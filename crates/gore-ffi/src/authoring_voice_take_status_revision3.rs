//! Prepare-only review-status editing for one existing revision-3 VoiceTake.
//!
//! This route accepts no game, save, Ogg-source, deployment, build, or fixed-head publication
//! authority. It fully opens the exact current Store project and every referenced asset, binds a
//! canonical status-edit request to that basis, evaluates the pure transaction, independently
//! reconstructs its only permitted delta, and fully reopens an immutable candidate checkpoint.
//! The fixed `gore-project.json` head is checked before preparation, after preparation, and after
//! response construction, but is never replaced here.

use std::path::Path;

use gore_authoring::model_revision3::{EntityKind, EntityPayload, VoiceTakeStatus};
use gore_authoring::{
    apply_revision3_voice_take_status_edit_transaction_v1,
    validate_revision3_voice_loc_id_basename_stem_v1, AssetVerification, EntityId,
    ProjectRevision3, Revision3VoiceTakeStatusEditBuildStatusV1,
    Revision3VoiceTakeStatusEditConflictV1, Revision3VoiceTakeStatusEditErrorV1,
    Revision3VoiceTakeStatusEditEvaluationV1, Revision3VoiceTakeStatusEditRequestV1,
    Revision3VoiceTakeStatusEditRuntimeStatusV1, WorkingHead, WorkingProjectStore,
    WorkingStoreError, WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
    MAX_REVISION3_VOICE_TAKE_STATUS_EDIT_REQUEST_JSON_BYTES_V1,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_prepare_revision3_voice_take_status_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
// Project and VoiceTake revisions increment exactly once. Studio transports signed 64-bit JSON
// integers on every supported host language. VoiceSlot is returned unchanged and may use i64::MAX.
const MAX_INCREMENTABLE_REVISION: u64 = i64::MAX as u64 - 1;
const MAX_WIRE_REVISION: u64 = i64::MAX as u64;
// Canonical nested JSON strings need at most one extra escape byte per source byte. Store roots
// are arbitrary caller strings and retain the full six-byte JSON escape allowance.
const MAX_WIRE_BYTES: usize = MAX_PROJECT_JSON_BYTES * 2
    + MAX_REVISION3_VOICE_TAKE_STATUS_EDIT_REQUEST_JSON_BYTES_V1 * 2
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
struct PrepareVoiceTakeStatusWirePayload {
    current_project_json: String,
    root: String,
    voice_take_status_request_json: String,
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
struct BoundVoiceTakeStatus {
    localization_id: EntityId,
    slot_revision: u64,
    take_revision: u64,
    previous_status: VoiceTakeStatus,
}

pub(super) fn prepare_revision3_voice_take_status_v1_raw(input: &str) -> Value {
    prepare_revision3_voice_take_status_v1_inner(input).unwrap_or_else(Failure::response)
}

fn prepare_revision3_voice_take_status_v1_inner(input: &str) -> Result<Value, Failure> {
    prepare_revision3_voice_take_status_v1_inner_with_test_seams(input, || {}, || {}, || {})
}

#[cfg(test)]
fn prepare_revision3_voice_take_status_v1_inner_with_pre_prepare_guard<F>(
    input: &str,
    pre_prepare_guard: F,
) -> Result<Value, Failure>
where
    F: FnOnce(),
{
    prepare_revision3_voice_take_status_v1_inner_with_test_seams(
        input,
        pre_prepare_guard,
        || {},
        || {},
    )
}

#[cfg(test)]
fn prepare_revision3_voice_take_status_v1_inner_with_post_prepare_guard<F>(
    input: &str,
    post_prepare_guard: F,
) -> Result<Value, Failure>
where
    F: FnOnce(),
{
    prepare_revision3_voice_take_status_v1_inner_with_test_seams(
        input,
        || {},
        post_prepare_guard,
        || {},
    )
}

#[cfg(test)]
fn prepare_revision3_voice_take_status_v1_inner_with_final_guard<F>(
    input: &str,
    final_guard: F,
) -> Result<Value, Failure>
where
    F: FnOnce(),
{
    prepare_revision3_voice_take_status_v1_inner_with_test_seams(input, || {}, || {}, final_guard)
}

fn prepare_revision3_voice_take_status_v1_inner_with_test_seams<B, A, F>(
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
    let payload: PrepareVoiceTakeStatusWirePayload = parse_exact_wire(input)?;
    validate_payload(&payload)?;

    // Parse and close the bounded semantic request before opening or probing the Store root.
    let request =
        Revision3VoiceTakeStatusEditRequestV1::from_json(&payload.voice_take_status_request_json)
            .map_err(map_request_error)?;
    require_signed_serializable(&request)?;
    validate_request_shape(&request)?;

    let store = WorkingProjectStore::open_existing(Path::new(&payload.root), ffi_store_limits())
        .map_err(map_store_error)?;
    let basis = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    validate_basis_revisions(&basis.project, &request)?;
    require_signed_serializable(&basis.project)?;
    require_signed_serializable(&basis.head)?;

    let canonical_basis = basis.project.to_canonical_json().map_err(|_| {
        invariant("the exact current revision-3 project could not be serialized canonically")
    })?;
    if canonical_basis.as_bytes() != payload.current_project_json.as_bytes() {
        return Err(project_conflict(
            "current_project_json differs from the exact published revision-3 project",
        ));
    }

    let bound = bind_request_to_basis(&basis.head, &basis.project, &request)?;
    let outcome = match apply_revision3_voice_take_status_edit_transaction_v1(
        &basis.head,
        &canonical_basis,
        &payload.voice_take_status_request_json,
    )
    .map_err(map_transaction_error)?
    {
        Revision3VoiceTakeStatusEditEvaluationV1::Applied(outcome) => *outcome,
        Revision3VoiceTakeStatusEditEvaluationV1::Rejected(rejection) => {
            return Err(map_transaction_conflict(rejection.conflict));
        }
    };

    require_signed_serializable(&outcome.project)?;
    verify_outcome_binding(&basis.head, &basis.project, &request, bound, &outcome)?;
    match outcome.build_status {
        Revision3VoiceTakeStatusEditBuildStatusV1::Blocked => {}
    }
    match outcome.runtime_status {
        Revision3VoiceTakeStatusEditRuntimeStatusV1::RuntimeUnqualified => {}
    }

    // A publisher racing after the initial full open must be observed before any candidate is
    // prepared. Preparation is immutable, but even unreachable garbage should be avoided here.
    before_checkpoint();
    require_fixed_basis(&store, &basis.head, &basis.project)?;
    let prepared = store
        .prepare_revision3_checkpoint(Some(&basis.head), &outcome.project)
        .map_err(map_store_error)?;
    let reopened = store
        .open_revision3_head_bytes(&prepared.head_bytes, AssetVerification::Full)
        .map_err(map_store_error)?;
    if reopened.head != prepared.head || reopened.project != outcome.project {
        return Err(invariant(
            "the prepared Voice take status checkpoint did not fully reopen exactly",
        ));
    }
    let reopened_json = reopened.project.to_canonical_json().map_err(|_| {
        invariant("the fully reopened Voice take status candidate could not be serialized")
    })?;
    if reopened_json != outcome.canonical_project_json {
        return Err(invariant(
            "the fully reopened Voice take status candidate changed canonical bytes",
        ));
    }

    after_checkpoint();
    require_fixed_basis(&store, &basis.head, &basis.project)?;

    let basis_head_json = canonical_head_json(&basis.head)?;
    let candidate_head_json = String::from_utf8(prepared.head_bytes)
        .map_err(|_| invariant("the prepared Voice take status head is not UTF-8 JSON"))?;
    if candidate_head_json.is_empty() || candidate_head_json.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_RESPONSE_LIMIT",
            "the prepared Voice take status head exceeds its bounded transport limit",
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
        "localization_id": outcome.localization_id.to_string(),
        "slot_id": outcome.slot_id.to_string(),
        "slot_revision": outcome.slot_revision,
        "locale": outcome.locale.to_string(),
        "loc_id": outcome.loc_id,
        "take_id": outcome.take_id.to_string(),
        "take_revision": outcome.take_revision,
        "previous_status": status_wire(outcome.previous_status),
        "status": status_wire(outcome.status),
        "build_status": "blocked",
        "runtime_status": "runtime_unqualified",
        "publication_status": "not_supported",
    });
    enforce_response_budget(&response)?;

    // A publisher after response construction remains authoritative. This never restores basis.
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
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_INPUT_LIMIT",
            format!(
                "revision-3 Voice take status request exceeds the {MAX_WIRE_BYTES}-byte wire limit"
            ),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    let canonical = serde_json::to_string(&request)
        .map_err(|_| invariant("the Voice take status outer request could not be serialized"))?;
    if canonical.as_bytes() != input.as_bytes() {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_payload(payload: &PrepareVoiceTakeStatusWirePayload) -> Result<(), Failure> {
    validate_path(&payload.root)?;
    if payload.current_project_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_PROJECT_LIMIT",
            format!("current_project_json exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ));
    }
    if payload.voice_take_status_request_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.voice_take_status_request_json.len()
        > MAX_REVISION3_VOICE_TAKE_STATUS_EDIT_REQUEST_JSON_BYTES_V1
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_REQUEST_LIMIT",
            format!(
                "voice_take_status_request_json exceeds the {MAX_REVISION3_VOICE_TAKE_STATUS_EDIT_REQUEST_JSON_BYTES_V1}-byte limit"
            ),
        ));
    }
    Ok(())
}

fn validate_request_shape(request: &Revision3VoiceTakeStatusEditRequestV1) -> Result<(), Failure> {
    let identities = [
        request.line_id,
        request.localization_id,
        request.slot_id,
        request.take_id,
    ];
    if identities.iter().copied().any(is_zero_entity_id)
        || identities
            .iter()
            .enumerate()
            .any(|(index, id)| identities[index + 1..].contains(id))
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_REQUEST_REJECTED",
            "line, localization, VoiceSlot, and VoiceTake identities must be non-zero and distinct",
        ));
    }
    if validate_revision3_voice_loc_id_basename_stem_v1(&request.expected_loc_id).is_err() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_REQUEST_REJECTED",
            "expected_loc_id is not one bounded portable Voice basename stem",
        ));
    }
    Ok(())
}

fn validate_basis_revisions(
    project: &ProjectRevision3,
    request: &Revision3VoiceTakeStatusEditRequestV1,
) -> Result<(), Failure> {
    if project.revision > MAX_INCREMENTABLE_REVISION {
        return Err(revision_limit(
            "the published project revision cannot be incremented on the signed wire",
        ));
    }
    if let Some(slot) = project.entities.get(&request.slot_id) {
        if slot.revision > MAX_WIRE_REVISION {
            return Err(revision_limit(
                "the published VoiceSlot revision exceeds the signed wire",
            ));
        }
    }
    if let Some(take) = project.entities.get(&request.take_id) {
        if take.revision > MAX_INCREMENTABLE_REVISION {
            return Err(revision_limit(
                "the published VoiceTake revision cannot be incremented on the signed wire",
            ));
        }
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
    request: &Revision3VoiceTakeStatusEditRequestV1,
) -> Result<BoundVoiceTakeStatus, Failure> {
    if request.expected_head != *head {
        return Err(head_conflict());
    }
    if request.expected_project_id != project.project_id
        || request.expected_revision != project.revision
    {
        return Err(project_conflict(
            "the Voice take status request project differs from the exact published project",
        ));
    }
    if request.expected_target != project.target {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_TARGET_CONFLICT",
            "the Voice take status request target differs from the exact published target",
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
        || line.localization.id != request.localization_id
    {
        return Err(line_conflict(
            "the DialogLine references a different exact-project LocalizationEntry",
        ));
    }
    let localization_entity = project
        .entities
        .get(&request.localization_id)
        .ok_or_else(|| line_conflict("the exact DialogLine LocalizationEntry is missing"))?;
    let EntityPayload::LocalizationEntry(localization) = &localization_entity.payload else {
        return Err(line_conflict(
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
    if !has_unique_slot_owner(project, request.line_id, &request.locale, request.slot_id) {
        return Err(slot_conflict(
            "the requested VoiceSlot does not have one exact line/locale owner",
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
            "the VoiceSlot revision differs from the exact published entity revision",
        ));
    }
    if slot.locale != request.locale {
        return Err(slot_conflict(
            "the VoiceSlot locale differs from the exact DialogLine locale",
        ));
    }
    let candidate_count = slot
        .candidates
        .iter()
        .filter(|candidate| {
            candidate.project_id == project.project_id
                && candidate.expected_kind == EntityKind::VoiceTake
                && candidate.id == request.take_id
        })
        .count();
    if candidate_count != 1 || !has_unique_take_owner(project, request.slot_id, request.take_id) {
        return Err(take_conflict(
            "the requested VoiceTake is not one uniquely retained candidate of the VoiceSlot",
        ));
    }

    let take_entity = project.entities.get(&request.take_id).ok_or_else(|| {
        take_conflict("the requested VoiceTake is missing from the exact published project")
    })?;
    let EntityPayload::VoiceTake(take) = &take_entity.payload else {
        return Err(take_conflict(
            "the requested entity is not a VoiceTake in the exact published project",
        ));
    };
    if take.locale != request.locale {
        return Err(take_conflict(
            "the requested VoiceTake locale differs from its exact VoiceSlot",
        ));
    }
    if take_entity.revision != request.expected_take_revision {
        return Err(take_conflict(
            "the VoiceTake revision differs from the exact published entity revision",
        ));
    }
    if take.status != request.expected_status {
        return Err(status_conflict(
            "the VoiceTake status differs from the exact published status",
        ));
    }
    let take_is_selected = slot.selected.as_ref().is_some_and(|selected| {
        selected.project_id == project.project_id
            && selected.expected_kind == EntityKind::VoiceTake
            && selected.id == request.take_id
    });
    if take_is_selected
        && request.desired_status != take.status
        && request.desired_status != VoiceTakeStatus::Approved
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_SELECTED_CONFLICT",
            "clear the selected VoiceTake before changing it to a non-Approved status",
        ));
    }
    Ok(BoundVoiceTakeStatus {
        localization_id: request.localization_id,
        slot_revision: slot_entity.revision,
        take_revision: take_entity.revision,
        previous_status: take.status,
    })
}

fn verify_outcome_binding(
    basis_head: &WorkingHead,
    basis: &ProjectRevision3,
    request: &Revision3VoiceTakeStatusEditRequestV1,
    bound: BoundVoiceTakeStatus,
    outcome: &gore_authoring::Revision3VoiceTakeStatusEditOutcomeV1,
) -> Result<(), Failure> {
    let expected_revision = basis
        .revision
        .checked_add(1)
        .ok_or_else(|| revision_limit("the project revision cannot be incremented"))?;
    let expected_take_revision = bound
        .take_revision
        .checked_add(1)
        .ok_or_else(|| revision_limit("the VoiceTake revision cannot be incremented"))?;
    if outcome.basis_head != *basis_head
        || outcome.line_id != request.line_id
        || outcome.localization_id != bound.localization_id
        || outcome.slot_id != request.slot_id
        || outcome.slot_revision != bound.slot_revision
        || outcome.locale != request.locale
        || outcome.loc_id != request.expected_loc_id
        || outcome.take_id != request.take_id
        || outcome.take_revision != expected_take_revision
        || outcome.previous_status != bound.previous_status
        || outcome.status != request.desired_status
        || outcome.project.project_id != basis.project_id
        || outcome.project.target != basis.target
        || outcome.project.revision != expected_revision
    {
        return Err(invariant(
            "the Voice take status transaction changed its exact project/request binding",
        ));
    }

    // Close the FFI boundary independently: only project revision, VoiceTake revision, and that
    // take's status may differ. Slot selection/order and every asset byte remain unchanged.
    let mut expected = basis.clone();
    expected.revision = expected_revision;
    let expected_take_entity = expected.entities.get_mut(&request.take_id).ok_or_else(|| {
        invariant("the bound VoiceTake disappeared while closing the candidate delta")
    })?;
    expected_take_entity.revision = expected_take_revision;
    let EntityPayload::VoiceTake(expected_take) = &mut expected_take_entity.payload else {
        return Err(invariant(
            "the bound VoiceTake changed kind while closing the candidate delta",
        ));
    };
    expected_take.status = request.desired_status;
    let expected_json = expected.to_canonical_json().map_err(|_| {
        invariant("the independently reconstructed Voice take status candidate is invalid")
    })?;
    if outcome.project != expected || outcome.canonical_project_json != expected_json {
        return Err(invariant(
            "the Voice take status transaction changed content outside the bound take status",
        ));
    }
    Ok(())
}

fn has_unique_slot_owner(
    project: &ProjectRevision3,
    expected_line: EntityId,
    expected_locale: &gore_authoring::LocaleCode,
    slot_id: EntityId,
) -> bool {
    let mut owner = None;
    for (line_id, entity) in &project.entities {
        let EntityPayload::DialogLine(line) = &entity.payload else {
            continue;
        };
        for (locale, reference) in &line.voice_slots {
            if reference.project_id == project.project_id
                && reference.expected_kind == EntityKind::VoiceSlot
                && reference.id == slot_id
                && owner.replace((*line_id, locale)).is_some()
            {
                return false;
            }
        }
    }
    matches!(owner, Some((line, locale)) if line == expected_line && locale == expected_locale)
}

fn has_unique_take_owner(
    project: &ProjectRevision3,
    expected_slot: EntityId,
    take_id: EntityId,
) -> bool {
    let mut owner = None;
    for (slot_id, entity) in &project.entities {
        let EntityPayload::VoiceSlot(slot) = &entity.payload else {
            continue;
        };
        for candidate in &slot.candidates {
            if candidate.project_id == project.project_id
                && candidate.expected_kind == EntityKind::VoiceTake
                && candidate.id == take_id
                && owner.replace(*slot_id).is_some()
            {
                return false;
            }
        }
    }
    owner == Some(expected_slot)
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

fn require_signed_serializable(value: &impl Serialize) -> Result<(), Failure> {
    let value = serde_json::to_value(value)
        .map_err(|_| invariant("a Voice take status wire value could not be inspected"))?;
    require_signed_json_value(&value)
}

fn require_signed_json_value(value: &Value) -> Result<(), Failure> {
    match value {
        Value::Number(number) if number.as_u64().is_some_and(|value| value > i64::MAX as u64) => {
            Err(Failure::new(
                "AUTHORING_REVISION3_VOICE_TAKE_STATUS_SIGNED_WIRE_LIMIT",
                "a Voice take status wire integer exceeds the signed 64-bit transport range",
            ))
        }
        Value::Array(values) => values.iter().try_for_each(require_signed_json_value),
        Value::Object(values) => values.values().try_for_each(require_signed_json_value),
        _ => Ok(()),
    }
}

fn canonical_head_json(head: &WorkingHead) -> Result<String, Failure> {
    require_signed_serializable(head)?;
    let value = serde_json::to_string(head)
        .map_err(|_| invariant("the Voice take status basis head could not be serialized"))?;
    if value.is_empty() || value.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_RESPONSE_LIMIT",
            "the Voice take status basis head exceeds its bounded transport limit",
        ));
    }
    Ok(value)
}

fn enforce_response_budget(response: &Value) -> Result<(), Failure> {
    let encoded = serde_json::to_vec(response)
        .map_err(|_| invariant("the Voice take status response could not be serialized"))?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_RESPONSE_LIMIT",
            "the Voice take status response exceeds its bounded transport budget",
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

fn status_wire(status: VoiceTakeStatus) -> &'static str {
    match status {
        VoiceTakeStatus::Draft => "draft",
        VoiceTakeStatus::Recorded => "recorded",
        VoiceTakeStatus::Reviewed => "reviewed",
        VoiceTakeStatus::Approved => "approved",
    }
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TAKE_STATUS_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and exactly current_project_json, root, and voice_take_status_request_json",
    )
}

fn invariant(message: &'static str) -> Failure {
    Failure::new("AUTHORING_REVISION3_VOICE_TAKE_STATUS_INVARIANT", message)
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TAKE_STATUS_HEAD_CONFLICT",
        "the published revision-3 head changed or differs from the Voice take status request",
    )
}

fn project_conflict(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TAKE_STATUS_PROJECT_CONFLICT",
        message,
    )
}

fn line_conflict(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TAKE_STATUS_LINE_CONFLICT",
        message,
    )
}

fn slot_conflict(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TAKE_STATUS_SLOT_CONFLICT",
        message,
    )
}

fn take_conflict(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TAKE_STATUS_TAKE_CONFLICT",
        message,
    )
}

fn status_conflict(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TAKE_STATUS_STATUS_CONFLICT",
        message,
    )
}

fn project_invalid(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TAKE_STATUS_PROJECT_INVALID",
        message,
    )
}

fn revision_limit(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TAKE_STATUS_REVISION_LIMIT",
        message,
    )
}

fn map_request_error(_error: impl std::fmt::Display) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TAKE_STATUS_REQUEST_INVALID",
        "the exact canonical Voice take status request is invalid",
    )
}

fn map_transaction_error(error: Revision3VoiceTakeStatusEditErrorV1) -> Failure {
    match error {
        Revision3VoiceTakeStatusEditErrorV1::InvalidProject(_) => project_invalid(
            "the exact current revision-3 project is not a valid Voice take status basis",
        ),
        Revision3VoiceTakeStatusEditErrorV1::InvalidRequest(error) => map_request_error(error),
        Revision3VoiceTakeStatusEditErrorV1::ReopenCandidate(_)
        | Revision3VoiceTakeStatusEditErrorV1::CanonicalReopenMismatch => {
            invariant("the Voice take status candidate failed exact canonical reopen")
        }
    }
}

fn map_transaction_conflict(error: Revision3VoiceTakeStatusEditConflictV1) -> Failure {
    let code = match &error {
        Revision3VoiceTakeStatusEditConflictV1::CurrentHeadMismatch => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_HEAD_CONFLICT"
        }
        Revision3VoiceTakeStatusEditConflictV1::ProjectIdentityMismatch { .. }
        | Revision3VoiceTakeStatusEditConflictV1::ProjectRevisionConflict { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_PROJECT_CONFLICT"
        }
        Revision3VoiceTakeStatusEditConflictV1::ProjectTargetMismatch => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_TARGET_CONFLICT"
        }
        Revision3VoiceTakeStatusEditConflictV1::ProjectRevisionOverflow
        | Revision3VoiceTakeStatusEditConflictV1::VoiceTakeRevisionOverflow { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_REVISION_LIMIT"
        }
        Revision3VoiceTakeStatusEditConflictV1::InvalidEntityIdentity
        | Revision3VoiceTakeStatusEditConflictV1::InvalidExpectedLocId => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_REQUEST_REJECTED"
        }
        Revision3VoiceTakeStatusEditConflictV1::InvalidDialogLine { .. }
        | Revision3VoiceTakeStatusEditConflictV1::LocalizationReferenceMismatch { .. }
        | Revision3VoiceTakeStatusEditConflictV1::InvalidLocalization { .. }
        | Revision3VoiceTakeStatusEditConflictV1::LocalizationIdentityMismatch { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_LINE_CONFLICT"
        }
        Revision3VoiceTakeStatusEditConflictV1::VoiceSlotIdentityMismatch { .. }
        | Revision3VoiceTakeStatusEditConflictV1::InvalidVoiceSlot { .. }
        | Revision3VoiceTakeStatusEditConflictV1::VoiceSlotRevisionConflict { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_SLOT_CONFLICT"
        }
        Revision3VoiceTakeStatusEditConflictV1::VoiceTakeNotCandidate { .. }
        | Revision3VoiceTakeStatusEditConflictV1::SharedVoiceTake { .. }
        | Revision3VoiceTakeStatusEditConflictV1::InvalidVoiceTake { .. }
        | Revision3VoiceTakeStatusEditConflictV1::VoiceTakeLocaleMismatch { .. }
        | Revision3VoiceTakeStatusEditConflictV1::VoiceTakeRevisionConflict { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_TAKE_CONFLICT"
        }
        Revision3VoiceTakeStatusEditConflictV1::CurrentStatusMismatch { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_STATUS_CONFLICT"
        }
        Revision3VoiceTakeStatusEditConflictV1::NoChanges => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_NO_CHANGES"
        }
        Revision3VoiceTakeStatusEditConflictV1::SelectedTakeCannotBecomeUnapproved { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_SELECTED_CONFLICT"
        }
        Revision3VoiceTakeStatusEditConflictV1::CandidateTooLarge { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_PROJECT_LIMIT"
        }
        Revision3VoiceTakeStatusEditConflictV1::CandidateNotPersistable { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_PROJECT_INVALID"
        }
    };
    Failure::new(code, error.to_string())
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::InvalidLimits(_) => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_STORE_LIMITS_INVALID"
        }
        WorkingStoreError::MissingRoot(_) => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_STORE_ROOT_MISSING"
        }
        WorkingStoreError::UnsafePath { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_STORE_PATH_UNSAFE"
        }
        WorkingStoreError::LimitExceeded { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_STORE_LIMIT"
        }
        WorkingStoreError::HeadConflict { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_HEAD_CONFLICT"
        }
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_VOICE_TAKE_STATUS_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_STORE_OBJECT_MISSING"
        }
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_STORE_COLLISION"
        }
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_STORE_IO"
        }
    };
    Failure::new(
        code,
        "the revision-3 Voice take status Store operation failed",
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
    const LOCALIZATION_ID_BYTE: u8 = 0x42;
    const LINE_ID_BYTE: u8 = 0x43;
    const SLOT_ID_BYTE: u8 = 0x44;
    const SELECTED_TAKE_ID_BYTE: u8 = 0x45;
    const EDIT_TAKE_ID_BYTE: u8 = 0x46;
    const SELECTED_TAKE_REVISION: u64 = 4;
    const EDIT_TAKE_REVISION: u64 = 5;

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
        ProjectId::from_bytes([0x51; 16])
    }

    fn locale() -> LocaleCode {
        "de".parse().unwrap()
    }

    fn target() -> GameGenerationAnchor {
        GameGenerationAnchor {
            executable: ContentSeal {
                byte_len: 171_698_176,
                sha256: Sha256Digest::from_bytes([0x61; 32]),
            },
        }
    }

    fn imported_origin(tag: u8) -> OriginRef {
        OriginRef::Imported {
            importer: "voice-take-status-ffi-tests".to_owned(),
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
                name: "Voice take status FFI fixture".to_owned(),
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
        Entity {
            id: id(tag),
            display_name: display_name.to_owned(),
            origin: imported_origin(tag),
            revision,
            payload,
        }
    }

    fn take_entity(
        tag: u8,
        revision: u64,
        suffix: &str,
        status: VoiceTakeStatus,
        imported: &gore_authoring::ImportedOgg,
    ) -> Entity {
        let mut asset = imported.asset.clone();
        asset.logical_name = format!("{LOC_ID}_{suffix}.ogg");
        entity(
            tag,
            revision,
            &format!("Asghan {suffix}"),
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
        let selected_take_id = id(SELECTED_TAKE_ID_BYTE);
        let edit_take_id = id(EDIT_TAKE_ID_BYTE);
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
                    2,
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
                        candidates: vec![take_ref(selected_take_id), take_ref(edit_take_id)],
                        selected: Some(take_ref(selected_take_id)),
                    }),
                ),
            ),
            (
                selected_take_id,
                take_entity(
                    SELECTED_TAKE_ID_BYTE,
                    SELECTED_TAKE_REVISION,
                    "selected",
                    VoiceTakeStatus::Approved,
                    imported,
                ),
            ),
            (
                edit_take_id,
                take_entity(
                    EDIT_TAKE_ID_BYTE,
                    EDIT_TAKE_REVISION,
                    "recorded",
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
        let source = source_temp.path().join("asghan-status-fixture.ogg");
        fs::write(
            &source,
            include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
        )
        .unwrap();
        let imported = store
            .import_ogg(&source, "asghan-status-fixture.ogg", Some(&basis.head))
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

    fn status_request(
        store: &PublishedStore,
        take_id: EntityId,
        take_revision: u64,
        expected_status: VoiceTakeStatus,
        desired_status: VoiceTakeStatus,
    ) -> Revision3VoiceTakeStatusEditRequestV1 {
        Revision3VoiceTakeStatusEditRequestV1 {
            expected_head: store.head.clone(),
            expected_project_id: store.project.project_id,
            expected_revision: store.project.revision,
            expected_target: store.project.target.clone(),
            line_id: id(LINE_ID_BYTE),
            localization_id: id(LOCALIZATION_ID_BYTE),
            expected_loc_id: LOC_ID.to_owned(),
            locale: locale(),
            slot_id: id(SLOT_ID_BYTE),
            expected_slot_revision: 3,
            take_id,
            expected_take_revision: take_revision,
            expected_status,
            desired_status,
        }
    }

    fn edit_request(store: &PublishedStore) -> Revision3VoiceTakeStatusEditRequestV1 {
        status_request(
            store,
            id(EDIT_TAKE_ID_BYTE),
            EDIT_TAKE_REVISION,
            VoiceTakeStatus::Recorded,
            VoiceTakeStatus::Approved,
        )
    }

    fn raw_request(payload: Value) -> String {
        serde_json::to_string(&json!({"command": COMMAND, "payload": payload})).unwrap()
    }

    fn wire(root: &Path, project_json: &str, request_json: &str) -> String {
        raw_request(json!({
            "current_project_json": project_json,
            "root": root,
            "voice_take_status_request_json": request_json,
        }))
    }

    fn call(store: &PublishedStore, request: &Revision3VoiceTakeStatusEditRequestV1) -> Value {
        prepare_revision3_voice_take_status_v1_raw(&wire(
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

    #[test]
    fn exact_wire_and_public_dispatch_reject_duplicates_unknown_authority_and_order() {
        let valid = raw_request(json!({
            "current_project_json": "{}",
            "root": "C:/missing",
            "voice_take_status_request_json": "{}",
        }));
        let parsed: PrepareVoiceTakeStatusWirePayload = parse_exact_wire(&valid).unwrap();
        assert_eq!(parsed.current_project_json, "{}");

        let cases = [
            format!(
                "{{\"command\":\"{COMMAND}\",\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"root\":\"r\",\"voice_take_status_request_json\":\"{{}}\"}}}}"
            ),
            format!(
                "{{\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"root\":\"r\",\"root\":\"forged\",\"voice_take_status_request_json\":\"{{}}\"}}}}"
            ),
            raw_request(json!({
                "current_project_json": "{}", "root": "r",
                "voice_take_status_request_json": "{}", "game_root": "forged"
            })),
            raw_request(json!({"root": "r", "voice_take_status_request_json": "{}"})),
            format!(" {valid}"),
            format!(
                "{{\"payload\":{{\"current_project_json\":\"{{}}\",\"root\":\"C:/missing\",\"voice_take_status_request_json\":\"{{}}\"}},\"command\":\"{COMMAND}\"}}"
            ),
        ];
        for input in cases {
            assert_eq!(
                prepare_revision3_voice_take_status_v1_raw(&input)["error"]["code"],
                "AUTHORING_REVISION3_VOICE_TAKE_STATUS_REQUEST_INVALID",
                "{input}"
            );
        }

        let duplicate = format!(
            "{{\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"root\":\"r\",\"root\":\"forged\",\"voice_take_status_request_json\":\"{{}}\"}}}}"
        );
        let response: Value = serde_json::from_str(&crate::execute_json(&duplicate)).unwrap();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_REQUEST_INVALID"
        );
    }

    #[test]
    fn malformed_nested_request_and_limits_reject_before_store_probe_or_write() {
        let missing_parent = TempDir::new().unwrap();
        let missing_root = missing_parent.path().join("missing-store");
        let response = prepare_revision3_voice_take_status_v1_raw(&wire(&missing_root, "{}", "{}"));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_REQUEST_INVALID"
        );
        assert!(!missing_root.exists());

        let store = published_store();
        let before = snapshot_regular_files(store.temp.path());
        let canonical = edit_request(&store).to_canonical_json().unwrap();
        for nested in [
            "{}".to_owned(),
            format!(" {canonical}"),
            r#"{"C:/secret/voice-status":true}"#.to_owned(),
            canonical.replacen(
                "\"expected_revision\":1",
                "\"expected_revision\":1,\"expected_revision\":1",
                1,
            ),
        ] {
            let response = prepare_revision3_voice_take_status_v1_raw(&wire(
                store.temp.path(),
                &store.project_json,
                &nested,
            ));
            assert_eq!(
                response["error"]["code"],
                "AUTHORING_REVISION3_VOICE_TAKE_STATUS_REQUEST_INVALID"
            );
            assert!(!response.to_string().contains("secret"));
        }

        let oversized = "x".repeat(MAX_REVISION3_VOICE_TAKE_STATUS_EDIT_REQUEST_JSON_BYTES_V1 + 1);
        assert_eq!(
            prepare_revision3_voice_take_status_v1_raw(&wire(
                store.temp.path(),
                &store.project_json,
                &oversized,
            ))["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_REQUEST_LIMIT"
        );
        assert_eq!(snapshot_regular_files(store.temp.path()), before);
    }

    #[test]
    fn recorded_to_approved_prepares_exact_fully_reopened_delta_without_publication() {
        let store = published_store();
        let response = call(&store, &edit_request(&store));
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["outcome"], "prepared_unpublished");
        assert_eq!(response["project_id"], store.project.project_id.to_string());
        assert_eq!(response["revision"], 2);
        assert_eq!(response["line_id"], id(LINE_ID_BYTE).to_string());
        assert_eq!(
            response["localization_id"],
            id(LOCALIZATION_ID_BYTE).to_string()
        );
        assert_eq!(response["slot_id"], id(SLOT_ID_BYTE).to_string());
        assert_eq!(response["slot_revision"], 3);
        assert_eq!(response["locale"], "de");
        assert_eq!(response["loc_id"], LOC_ID);
        assert_eq!(response["take_id"], id(EDIT_TAKE_ID_BYTE).to_string());
        assert_eq!(response["take_revision"], EDIT_TAKE_REVISION + 1);
        assert_eq!(response["previous_status"], "recorded");
        assert_eq!(response["status"], "approved");
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
                "localization_id",
                "loc_id",
                "locale",
                "ok",
                "outcome",
                "previous_status",
                "project_id",
                "project_json",
                "publication_status",
                "revision",
                "runtime_status",
                "slot_id",
                "slot_revision",
                "status",
                "take_id",
                "take_revision",
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
        let take_entity = expected.entities.get_mut(&id(EDIT_TAKE_ID_BYTE)).unwrap();
        take_entity.revision = EDIT_TAKE_REVISION + 1;
        let EntityPayload::VoiceTake(take) = &mut take_entity.payload else {
            panic!("expected VoiceTake")
        };
        take.status = VoiceTakeStatus::Approved;
        assert_eq!(reopened.project, expected);

        let encoded = response.to_string();
        assert!(!encoded.contains(store.temp.path().to_string_lossy().as_ref()));
        assert!(!encoded.contains("game_root"));
        assert!(!encoded.contains("\"source\":"));
        assert!(!encoded.contains("deploy"));
    }

    #[test]
    fn stale_graph_noop_and_selected_demotion_write_no_candidate_objects() {
        let store = published_store();
        let before = snapshot_regular_files(store.temp.path());
        let base = edit_request(&store);

        let response = prepare_revision3_voice_take_status_v1_raw(&wire(
            store.temp.path(),
            "not-the-exact-current-project",
            &base.to_canonical_json().unwrap(),
        ));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_PROJECT_CONFLICT"
        );

        let mut cases = Vec::new();
        let mut stale_head = base.clone();
        stale_head.expected_head.snapshot.byte_len += 1;
        cases.push((
            stale_head,
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_HEAD_CONFLICT",
        ));
        let mut foreign_project = base.clone();
        foreign_project.expected_project_id = ProjectId::from_bytes([0x71; 16]);
        cases.push((
            foreign_project,
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_PROJECT_CONFLICT",
        ));
        let mut stale_project = base.clone();
        stale_project.expected_revision = 0;
        cases.push((
            stale_project,
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_PROJECT_CONFLICT",
        ));
        let mut wrong_target = base.clone();
        wrong_target.expected_target.executable.sha256 = Sha256Digest::from_bytes([0x72; 32]);
        cases.push((
            wrong_target,
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_TARGET_CONFLICT",
        ));
        let mut wrong_line = base.clone();
        wrong_line.line_id = id(0x73);
        cases.push((
            wrong_line,
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_LINE_CONFLICT",
        ));
        let mut wrong_localization = base.clone();
        wrong_localization.localization_id = id(0x74);
        cases.push((
            wrong_localization,
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_LINE_CONFLICT",
        ));
        let mut wrong_loc_id = base.clone();
        wrong_loc_id.expected_loc_id = "GRD_OTHER_LINE".to_owned();
        cases.push((
            wrong_loc_id,
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_LINE_CONFLICT",
        ));
        let mut wrong_locale = base.clone();
        wrong_locale.locale = "en".parse().unwrap();
        cases.push((
            wrong_locale,
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_SLOT_CONFLICT",
        ));
        let mut wrong_slot = base.clone();
        wrong_slot.slot_id = id(0x75);
        cases.push((
            wrong_slot,
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_SLOT_CONFLICT",
        ));
        let mut stale_slot = base.clone();
        stale_slot.expected_slot_revision = 2;
        cases.push((
            stale_slot,
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_SLOT_CONFLICT",
        ));
        let mut wrong_take = base.clone();
        wrong_take.take_id = id(0x76);
        cases.push((
            wrong_take,
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_TAKE_CONFLICT",
        ));
        let mut stale_take = base.clone();
        stale_take.expected_take_revision = EDIT_TAKE_REVISION - 1;
        cases.push((
            stale_take,
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_TAKE_CONFLICT",
        ));
        let mut stale_status = base.clone();
        stale_status.expected_status = VoiceTakeStatus::Reviewed;
        cases.push((
            stale_status,
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_STATUS_CONFLICT",
        ));
        let mut noop = base.clone();
        noop.desired_status = VoiceTakeStatus::Recorded;
        cases.push((noop, "AUTHORING_REVISION3_VOICE_TAKE_STATUS_NO_CHANGES"));
        let selected_demotion = status_request(
            &store,
            id(SELECTED_TAKE_ID_BYTE),
            SELECTED_TAKE_REVISION,
            VoiceTakeStatus::Approved,
            VoiceTakeStatus::Reviewed,
        );
        cases.push((
            selected_demotion,
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_SELECTED_CONFLICT",
        ));
        let mut zero = base.clone();
        zero.line_id = EntityId::from_bytes([0; 16]);
        cases.push((
            zero,
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_REQUEST_REJECTED",
        ));
        let mut invalid_loc = base;
        invalid_loc.expected_loc_id = "C:/secret/voice".to_owned();
        cases.push((
            invalid_loc,
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_REQUEST_REJECTED",
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
    fn signed_and_increment_revision_limits_reject_before_candidate_write() {
        let store = published_store();
        let before = snapshot_regular_files(store.temp.path());
        let mut request = edit_request(&store);
        request.expected_revision = u64::MAX;
        assert_eq!(
            call(&store, &request)["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_SIGNED_WIRE_LIMIT"
        );
        assert_eq!(snapshot_regular_files(store.temp.path()), before);

        let mut store = store;
        let working =
            WorkingProjectStore::open_existing(store.temp.path(), ffi_store_limits()).unwrap();
        let mut project = store.project.clone();
        project.revision += 1;
        project
            .entities
            .get_mut(&id(EDIT_TAKE_ID_BYTE))
            .unwrap()
            .revision = MAX_INCREMENTABLE_REVISION + 1;
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
        let mut request = edit_request(&store);
        request.expected_take_revision = MAX_INCREMENTABLE_REVISION + 1;
        let response = call(&store, &request);
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_REVISION_LIMIT"
        );
        assert_eq!(snapshot_regular_files(store.temp.path()), before);
    }

    #[test]
    fn all_three_head_race_gates_preserve_external_publishers_and_never_restore_basis() {
        for gate in ["pre", "post", "final"] {
            let store = published_store();
            let rival = rival_head(&store, &format!("External {gate} publisher"));
            let request = edit_request(&store);
            let wire = wire(
                store.temp.path(),
                &store.project_json,
                &request.to_canonical_json().unwrap(),
            );
            let publish = || {
                fs::write(store.temp.path().join("gore-project.json"), &rival).unwrap();
            };
            let response = match gate {
                "pre" => prepare_revision3_voice_take_status_v1_inner_with_pre_prepare_guard(
                    &wire, publish,
                ),
                "post" => prepare_revision3_voice_take_status_v1_inner_with_post_prepare_guard(
                    &wire, publish,
                ),
                "final" => {
                    prepare_revision3_voice_take_status_v1_inner_with_final_guard(&wire, publish)
                }
                _ => unreachable!(),
            }
            .unwrap_err()
            .response();
            assert_eq!(
                response["error"]["code"], "AUTHORING_REVISION3_VOICE_TAKE_STATUS_HEAD_CONFLICT",
                "{gate}: {response}"
            );
            assert_eq!(fixed_head(&store), rival);
        }
    }

    #[test]
    fn full_asset_failure_is_path_free_and_never_publishes_candidate() {
        let store = published_store();
        let request = edit_request(&store);
        let wire = wire(
            store.temp.path(),
            &store.project_json,
            &request.to_canonical_json().unwrap(),
        );
        let response =
            prepare_revision3_voice_take_status_v1_inner_with_pre_prepare_guard(&wire, || {
                fs::remove_file(&store.asset_path).unwrap()
            })
            .unwrap_err()
            .response();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_STORE_OBJECT_MISSING"
        );
        assert!(!response
            .to_string()
            .contains(store.temp.path().to_string_lossy().as_ref()));
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
    }

    #[test]
    fn error_taxonomy_keeps_limits_distinct_and_hides_store_paths() {
        assert_eq!(
            map_transaction_conflict(Revision3VoiceTakeStatusEditConflictV1::NoChanges).code,
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_NO_CHANGES"
        );
        assert_eq!(
            map_transaction_conflict(Revision3VoiceTakeStatusEditConflictV1::CandidateTooLarge {
                actual: MAX_PROJECT_JSON_BYTES + 1,
                limit: MAX_PROJECT_JSON_BYTES,
            })
            .code,
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_PROJECT_LIMIT"
        );
        let unsafe_path = map_store_error(WorkingStoreError::UnsafePath {
            path: PathBuf::from("C:/secret/voice-status-store"),
            reason: "fixture".to_owned(),
        });
        assert_eq!(
            unsafe_path.code,
            "AUTHORING_REVISION3_VOICE_TAKE_STATUS_STORE_PATH_UNSAFE"
        );
        assert!(!unsafe_path.message.contains("secret"));
    }
}
