//! Prepare-only removal of one revision-3 VoiceTake candidate from one exact line/locale slot.
//!
//! The route accepts no game, save, source-media, build, deployment, runtime, blob-deletion, or
//! fixed-head publication authority. It fully opens the exact current Store project, binds the
//! request to the inspected graph, evaluates the pure transaction, independently reconstructs
//! the only permitted delta, and fully reopens an immutable candidate. Shared takes remain in
//! the project; final-use removal deliberately preserves the complete AssetStore and Ogg CAS.

use std::path::Path;

use gore_authoring::model_revision3::{EntityKind, EntityPayload, VoiceSlot};
use gore_authoring::{
    apply_revision3_voice_take_removal_transaction_v1, build_revision3_content_index_v1,
    validate_revision3_voice_loc_id_basename_stem_v1, AssetVerification, EntityId, LocaleCode,
    ProjectRevision3, Revision3ContentReferenceResolutionV1, Revision3ContentReferenceRoleV1,
    Revision3TypedRef, Revision3VoiceTakeRemovalBuildStatusV1, Revision3VoiceTakeRemovalConflictV1,
    Revision3VoiceTakeRemovalErrorV1, Revision3VoiceTakeRemovalEvaluationV1,
    Revision3VoiceTakeRemovalOutcomeV1, Revision3VoiceTakeRemovalRequestV1,
    Revision3VoiceTakeRemovalRuntimeStatusV1, WorkingHead, WorkingProjectStore, WorkingStoreError,
    WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
    MAX_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_JSON_BYTES_V1,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_prepare_revision3_voice_take_removal_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_INCREMENTABLE_REVISION: u64 = i64::MAX as u64 - 1;
const MAX_WIRE_REVISION: u64 = i64::MAX as u64;
const MAX_WIRE_BYTES: usize = MAX_PROJECT_JSON_BYTES * 2
    + MAX_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_JSON_BYTES_V1 * 2
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
struct PrepareVoiceTakeRemovalWirePayload {
    current_project_json: String,
    root: String,
    voice_take_removal_request_json: String,
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
struct BoundVoiceTakeRemoval {
    localization_id: EntityId,
    slot_revision: u64,
    take_revision: u64,
    previous_selected_take_id: Option<EntityId>,
    selection_cleared: bool,
    take_entity_removed: bool,
    remaining_candidate_count: u64,
}

pub(super) fn prepare_revision3_voice_take_removal_v1_raw(input: &str) -> Value {
    prepare_revision3_voice_take_removal_v1_inner(input).unwrap_or_else(Failure::response)
}

fn prepare_revision3_voice_take_removal_v1_inner(input: &str) -> Result<Value, Failure> {
    prepare_revision3_voice_take_removal_v1_inner_with_test_seams(input, || {}, || {}, || {})
}

fn prepare_revision3_voice_take_removal_v1_inner_with_test_seams<B, A, F>(
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
    let payload: PrepareVoiceTakeRemovalWirePayload = parse_exact_wire(input)?;
    validate_payload(&payload)?;

    let request =
        Revision3VoiceTakeRemovalRequestV1::from_json(&payload.voice_take_removal_request_json)
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
    let outcome = match apply_revision3_voice_take_removal_transaction_v1(
        &basis.head,
        &canonical_basis,
        &payload.voice_take_removal_request_json,
    )
    .map_err(map_transaction_error)?
    {
        Revision3VoiceTakeRemovalEvaluationV1::Applied(outcome) => *outcome,
        Revision3VoiceTakeRemovalEvaluationV1::Rejected(rejection) => {
            return Err(map_transaction_conflict(rejection.conflict));
        }
    };

    require_signed_serializable(&outcome.project)?;
    verify_outcome_binding(&basis.head, &basis.project, &request, bound, &outcome)?;
    match outcome.build_status {
        Revision3VoiceTakeRemovalBuildStatusV1::Blocked => {}
    }
    match outcome.runtime_status {
        Revision3VoiceTakeRemovalRuntimeStatusV1::RuntimeUnqualified => {}
    }

    // Observe a publisher racing after the initial full open before creating even unreachable
    // immutable candidate objects.
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
            "the prepared Voice take removal checkpoint did not fully reopen exactly",
        ));
    }
    let reopened_json = reopened.project.to_canonical_json().map_err(|_| {
        invariant("the fully reopened Voice take removal candidate could not be serialized")
    })?;
    if reopened_json != outcome.canonical_project_json {
        return Err(invariant(
            "the fully reopened Voice take removal candidate changed canonical bytes",
        ));
    }

    after_checkpoint();
    require_fixed_basis(&store, &basis.head, &basis.project)?;

    let basis_head_json = canonical_head_json(&basis.head)?;
    let candidate_head_json = checkpoint_head_json(&prepared.head_bytes)?;
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
        "previous_selected_take_id": outcome
            .previous_selected_take_id
            .map(|id| id.to_string()),
        "selection_cleared": outcome.selection_cleared,
        "take_entity_removed": outcome.take_entity_removed,
        "remaining_candidate_count": outcome.remaining_candidate_count,
        "build_status": "blocked",
        "runtime_status": "runtime_unqualified",
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
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_INPUT_LIMIT",
            format!(
                "revision-3 Voice take removal request exceeds the {MAX_WIRE_BYTES}-byte wire limit"
            ),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    let canonical = serde_json::to_string(&request)
        .map_err(|_| invariant("the Voice take removal wire request could not be serialized"))?;
    if canonical.as_bytes() != input.as_bytes() {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_payload(payload: &PrepareVoiceTakeRemovalWirePayload) -> Result<(), Failure> {
    validate_path(&payload.root)?;
    if payload.current_project_json.is_empty() || payload.voice_take_removal_request_json.is_empty()
    {
        return Err(invalid_request());
    }
    if payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_PROJECT_LIMIT",
            format!("current_project_json exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ));
    }
    if payload.voice_take_removal_request_json.len()
        > MAX_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_JSON_BYTES_V1
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_LIMIT",
            format!(
                "voice_take_removal_request_json exceeds the {MAX_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_JSON_BYTES_V1}-byte limit"
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

fn validate_request_shape(request: &Revision3VoiceTakeRemovalRequestV1) -> Result<(), Failure> {
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
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_REJECTED",
            "line, localization, VoiceSlot, and VoiceTake identities must be non-zero and distinct",
        ));
    }
    if request
        .expected_selected_take_id
        .is_some_and(is_zero_entity_id)
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_REJECTED",
            "the expected selected VoiceTake identity must be non-zero",
        ));
    }
    if validate_revision3_voice_loc_id_basename_stem_v1(&request.expected_loc_id).is_err() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_REJECTED",
            "expected_loc_id is not one bounded portable Voice basename stem",
        ));
    }
    Ok(())
}

fn validate_basis_revisions(
    project: &ProjectRevision3,
    request: &Revision3VoiceTakeRemovalRequestV1,
) -> Result<(), Failure> {
    if project.revision > MAX_INCREMENTABLE_REVISION {
        return Err(revision_limit(
            "the published project revision cannot be incremented on the signed wire",
        ));
    }
    if let Some(slot) = project.entities.get(&request.slot_id) {
        if slot.revision > MAX_INCREMENTABLE_REVISION {
            return Err(revision_limit(
                "the published VoiceSlot revision cannot be incremented on the signed wire",
            ));
        }
    }
    if let Some(take) = project.entities.get(&request.take_id) {
        if take.revision > MAX_WIRE_REVISION {
            return Err(revision_limit(
                "the published VoiceTake revision exceeds the signed wire",
            ));
        }
    }
    Ok(())
}

fn bind_request_to_basis(
    head: &WorkingHead,
    project: &ProjectRevision3,
    request: &Revision3VoiceTakeRemovalRequestV1,
) -> Result<BoundVoiceTakeRemoval, Failure> {
    if request.expected_head != *head {
        return Err(head_conflict());
    }
    if request.expected_project_id != project.project_id
        || request.expected_revision != project.revision
    {
        return Err(project_conflict(
            "the Voice take removal request project differs from the exact published project",
        ));
    }
    if request.expected_target != project.target {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_TARGET_CONFLICT",
            "the Voice take removal request target differs from the exact published target",
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
        return Err(localization_conflict(
            "the DialogLine references a different exact-project LocalizationEntry",
        ));
    }
    let localization_entity = project
        .entities
        .get(&request.localization_id)
        .ok_or_else(|| {
            localization_conflict("the exact DialogLine LocalizationEntry is missing")
        })?;
    let EntityPayload::LocalizationEntry(localization) = &localization_entity.payload else {
        return Err(localization_conflict(
            "the exact DialogLine localization reference has the wrong kind",
        ));
    };
    if localization.loc_id != request.expected_loc_id {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_LOC_ID_CONFLICT",
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
        .filter(|candidate| is_exact_take_ref(project, candidate, request.take_id))
        .count();
    if candidate_count != 1 {
        return Err(take_conflict(
            "the requested VoiceTake is not exactly one candidate of the requested VoiceSlot",
        ));
    }

    let previous_selected_take_id = exact_selected_take_id(project, slot)?;
    if previous_selected_take_id != request.expected_selected_take_id {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_SELECTION_CONFLICT",
            "the current VoiceSlot selection differs from the removal request",
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

    validate_take_backlinks(project, request.take_id)?;
    let mut expected = project.clone();
    let (selection_cleared, remaining_candidate_count) =
        remove_candidate_from_slot(&mut expected, request)?;
    let take_entity_removed = !has_permitted_local_take_use(&expected, request.take_id)?;

    Ok(BoundVoiceTakeRemoval {
        localization_id: request.localization_id,
        slot_revision: slot_entity.revision,
        take_revision: take_entity.revision,
        previous_selected_take_id,
        selection_cleared,
        take_entity_removed,
        remaining_candidate_count,
    })
}

fn verify_outcome_binding(
    basis_head: &WorkingHead,
    basis: &ProjectRevision3,
    request: &Revision3VoiceTakeRemovalRequestV1,
    bound: BoundVoiceTakeRemoval,
    outcome: &Revision3VoiceTakeRemovalOutcomeV1,
) -> Result<(), Failure> {
    let expected_revision = basis
        .revision
        .checked_add(1)
        .ok_or_else(|| revision_limit("the project revision cannot be incremented"))?;
    let expected_slot_revision = bound
        .slot_revision
        .checked_add(1)
        .ok_or_else(|| revision_limit("the VoiceSlot revision cannot be incremented"))?;
    if outcome.basis_head != *basis_head
        || outcome.line_id != request.line_id
        || outcome.localization_id != bound.localization_id
        || outcome.slot_id != request.slot_id
        || outcome.slot_revision != expected_slot_revision
        || outcome.locale != request.locale
        || outcome.loc_id != request.expected_loc_id
        || outcome.take_id != request.take_id
        || outcome.take_revision != bound.take_revision
        || outcome.previous_selected_take_id != bound.previous_selected_take_id
        || outcome.selection_cleared != bound.selection_cleared
        || outcome.take_entity_removed != bound.take_entity_removed
        || outcome.remaining_candidate_count != bound.remaining_candidate_count
        || outcome.project.project_id != basis.project_id
        || outcome.project.target != basis.target
        || outcome.project.revision != expected_revision
    {
        return Err(invariant(
            "the Voice take removal transaction changed its exact project/request binding",
        ));
    }

    // Close the FFI boundary independently. Only the project/slot revisions, the target slot's
    // candidate list and optional selected reference, and (after final use) the take entity may
    // differ. This reconstruction starts from the exact fully verified basis, so AssetStore bytes
    // and every other entity remain exact by construction.
    let mut expected = basis.clone();
    expected.revision = expected_revision;
    let (selection_cleared, remaining_candidate_count) =
        remove_candidate_from_slot(&mut expected, request)?;
    if selection_cleared != bound.selection_cleared
        || remaining_candidate_count != bound.remaining_candidate_count
    {
        return Err(invariant(
            "the independently reconstructed VoiceSlot removal changed its bound facts",
        ));
    }
    let still_used = has_permitted_local_take_use(&expected, request.take_id)?;
    if still_used == bound.take_entity_removed {
        return Err(invariant(
            "the VoiceTake cleanup result disagrees with the remaining exact local uses",
        ));
    }
    if bound.take_entity_removed {
        let removed = expected.entities.remove(&request.take_id).ok_or_else(|| {
            invariant("the final-use VoiceTake disappeared before independent cleanup")
        })?;
        if removed.revision != bound.take_revision
            || !matches!(removed.payload, EntityPayload::VoiceTake(_))
        {
            return Err(invariant(
                "the independently removed final-use entity was not the bound VoiceTake",
            ));
        }
    }
    let expected_json = expected.to_canonical_json().map_err(|_| {
        invariant("the independently reconstructed Voice take removal candidate is invalid")
    })?;
    if outcome.project != expected || outcome.canonical_project_json != expected_json {
        return Err(invariant(
            "the Voice take removal transaction changed content outside its exact closure",
        ));
    }
    Ok(())
}

fn remove_candidate_from_slot(
    project: &mut ProjectRevision3,
    request: &Revision3VoiceTakeRemovalRequestV1,
) -> Result<(bool, u64), Failure> {
    let project_id = project.project_id;
    let slot_entity = project
        .entities
        .get_mut(&request.slot_id)
        .ok_or_else(|| invariant("the bound VoiceSlot disappeared while reconstructing removal"))?;
    slot_entity.revision = slot_entity
        .revision
        .checked_add(1)
        .ok_or_else(|| revision_limit("the VoiceSlot revision cannot be incremented"))?;
    let EntityPayload::VoiceSlot(slot) = &mut slot_entity.payload else {
        return Err(invariant(
            "the bound VoiceSlot changed kind while reconstructing removal",
        ));
    };
    let before = slot.candidates.len();
    slot.candidates.retain(|candidate| {
        !(candidate.project_id == project_id
            && candidate.expected_kind == EntityKind::VoiceTake
            && candidate.id == request.take_id)
    });
    if before.checked_sub(slot.candidates.len()) != Some(1) {
        return Err(invariant(
            "the bound VoiceSlot did not contain exactly one removable candidate",
        ));
    }
    let selection_cleared = slot.selected.as_ref().is_some_and(|selected| {
        selected.project_id == project_id
            && selected.expected_kind == EntityKind::VoiceTake
            && selected.id == request.take_id
    });
    if selection_cleared {
        slot.selected = None;
    }
    let remaining_candidate_count = u64::try_from(slot.candidates.len()).map_err(|_| {
        invariant("the remaining VoiceTake candidate count exceeds the bounded wire")
    })?;
    Ok((selection_cleared, remaining_candidate_count))
}

fn validate_take_backlinks(project: &ProjectRevision3, take_id: EntityId) -> Result<(), Failure> {
    let index = build_revision3_content_index_v1(project).map_err(|error| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REFERENCE_LIMIT",
            format!("the exact Voice graph cannot be indexed safely: {error}"),
        )
    })?;
    for source in &index.entities {
        for reference in &source.references {
            if reference.target.entity_id != take_id
                || reference.target.project_id != project.project_id
            {
                continue;
            }
            let permitted_role = matches!(
                reference.role,
                Revision3ContentReferenceRoleV1::VoiceCandidate
                    | Revision3ContentReferenceRoleV1::VoiceSelected
            );
            if !permitted_role
                || source.kind != EntityKind::VoiceSlot
                || reference.qualifier.is_some()
                || reference.target.expected_kind != EntityKind::VoiceTake
                || reference.resolution != Revision3ContentReferenceResolutionV1::Resolved
            {
                return Err(Failure::new(
                    "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_BACKLINK_CONFLICT",
                    "the VoiceTake has an unresolved, kind-mismatched, qualified, or unexpected same-project backlink",
                ));
            }
            if reference.role == Revision3ContentReferenceRoleV1::VoiceSelected {
                let Some(entity) = project.entities.get(&source.id) else {
                    return Err(invariant(
                        "the Voice content index names a missing selected-take source",
                    ));
                };
                let EntityPayload::VoiceSlot(slot) = &entity.payload else {
                    return Err(invariant(
                        "the Voice content index selected-take source is not a VoiceSlot",
                    ));
                };
                if !slot
                    .candidates
                    .iter()
                    .any(|candidate| is_exact_take_ref(project, candidate, take_id))
                {
                    return Err(Failure::new(
                        "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_BACKLINK_CONFLICT",
                        "a selected VoiceTake is not also retained as a candidate by its VoiceSlot",
                    ));
                }
            }
        }
    }
    Ok(())
}

fn has_permitted_local_take_use(
    project: &ProjectRevision3,
    take_id: EntityId,
) -> Result<bool, Failure> {
    validate_take_backlinks(project, take_id)?;
    let index = build_revision3_content_index_v1(project).map_err(|error| {
        Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REFERENCE_LIMIT",
            format!("the exact Voice graph cannot be indexed safely: {error}"),
        )
    })?;
    Ok(index.entities.iter().any(|source| {
        source.references.iter().any(|reference| {
            reference.target.project_id == project.project_id
                && reference.target.entity_id == take_id
                && reference.target.expected_kind == EntityKind::VoiceTake
                && reference.resolution == Revision3ContentReferenceResolutionV1::Resolved
                && matches!(
                    reference.role,
                    Revision3ContentReferenceRoleV1::VoiceCandidate
                        | Revision3ContentReferenceRoleV1::VoiceSelected
                )
        })
    }))
}

fn exact_selected_take_id(
    project: &ProjectRevision3,
    slot: &VoiceSlot,
) -> Result<Option<EntityId>, Failure> {
    let Some(selected) = &slot.selected else {
        return Ok(None);
    };
    if selected.project_id != project.project_id
        || selected.expected_kind != EntityKind::VoiceTake
        || is_zero_entity_id(selected.id)
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_SELECTION_CONFLICT",
            "the exact VoiceSlot selection is not one local VoiceTake reference",
        ));
    }
    Ok(Some(selected.id))
}

fn is_exact_take_ref(
    project: &ProjectRevision3,
    reference: &Revision3TypedRef,
    take_id: EntityId,
) -> bool {
    reference.project_id == project.project_id
        && reference.expected_kind == EntityKind::VoiceTake
        && reference.id == take_id
}

fn has_unique_slot_owner(
    project: &ProjectRevision3,
    expected_line: EntityId,
    expected_locale: &LocaleCode,
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

fn checkpoint_head_json(head_bytes: &[u8]) -> Result<String, Failure> {
    if head_bytes.is_empty() || head_bytes.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_RESPONSE_LIMIT",
            "the prepared Voice take removal head exceeds its bounded transport limit",
        ));
    }
    let value = std::str::from_utf8(head_bytes).map_err(|_| {
        invariant("the prepared Voice take removal head is not canonical UTF-8 JSON")
    })?;
    let parsed: WorkingHead = serde_json::from_str(value)
        .map_err(|_| invariant("the prepared Voice take removal head is invalid JSON"))?;
    let canonical = canonical_head_json(&parsed)?;
    if canonical != value {
        return Err(invariant(
            "the prepared Voice take removal head is not in canonical spelling",
        ));
    }
    Ok(canonical)
}

fn canonical_head_json(head: &WorkingHead) -> Result<String, Failure> {
    require_signed_serializable(head)?;
    let value = serde_json::to_string(head)
        .map_err(|_| invariant("the Voice take removal head could not be serialized"))?;
    if value.is_empty() || value.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_RESPONSE_LIMIT",
            "the Voice take removal head exceeds its bounded transport limit",
        ));
    }
    Ok(value)
}

fn require_signed_serializable(value: &impl Serialize) -> Result<(), Failure> {
    let value = serde_json::to_value(value)
        .map_err(|_| invariant("a Voice take removal wire value could not be inspected"))?;
    require_signed_json_value(&value)
}

fn require_signed_json_value(value: &Value) -> Result<(), Failure> {
    match value {
        Value::Number(number) if number.as_u64().is_some_and(|value| value > i64::MAX as u64) => {
            Err(Failure::new(
                "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_SIGNED_WIRE_LIMIT",
                "a Voice take removal wire integer exceeds the signed 64-bit transport range",
            ))
        }
        Value::Array(values) => values.iter().try_for_each(require_signed_json_value),
        Value::Object(values) => values.values().try_for_each(require_signed_json_value),
        _ => Ok(()),
    }
}

fn enforce_response_budget(response: &Value) -> Result<(), Failure> {
    let encoded = serde_json::to_vec(response)
        .map_err(|_| invariant("the Voice take removal response could not be serialized"))?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_RESPONSE_LIMIT",
            "the Voice take removal response exceeds its bounded transport budget",
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
        "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and exactly current_project_json, root, and voice_take_removal_request_json",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_HEAD_CONFLICT",
        "the published revision-3 head changed or differs from the Voice take removal request",
    )
}

fn project_conflict(message: impl Into<String>) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_PROJECT_CONFLICT",
        message,
    )
}

fn line_conflict(message: impl Into<String>) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_LINE_CONFLICT",
        message,
    )
}

fn localization_conflict(message: impl Into<String>) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_LOCALIZATION_CONFLICT",
        message,
    )
}

fn slot_conflict(message: impl Into<String>) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_SLOT_CONFLICT",
        message,
    )
}

fn take_conflict(message: impl Into<String>) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_TAKE_CONFLICT",
        message,
    )
}

fn revision_limit(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REVISION_LIMIT",
        message,
    )
}

fn invariant(message: impl Into<String>) -> Failure {
    Failure::new("AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_INVARIANT", message)
}

fn map_request_error(error: impl std::fmt::Display) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_INVALID",
        format!("the exact Voice take removal request is invalid: {error}"),
    )
}

fn map_transaction_error(error: Revision3VoiceTakeRemovalErrorV1) -> Failure {
    match error {
        Revision3VoiceTakeRemovalErrorV1::InvalidProject(error) => Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_PROJECT_INVALID",
            format!("the exact current project is invalid: {error}"),
        ),
        Revision3VoiceTakeRemovalErrorV1::InvalidRequest(error) => map_request_error(error),
        Revision3VoiceTakeRemovalErrorV1::ContentIndex(error) => Failure::new(
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_PROJECT_INVALID",
            format!("the exact project content index is invalid: {error}"),
        ),
        Revision3VoiceTakeRemovalErrorV1::ReopenCandidate(_)
        | Revision3VoiceTakeRemovalErrorV1::CanonicalReopenMismatch
        | Revision3VoiceTakeRemovalErrorV1::CandidatePreservationMismatch => {
            invariant("the pure Voice take removal candidate failed canonical preservation")
        }
    }
}

fn map_transaction_conflict(error: Revision3VoiceTakeRemovalConflictV1) -> Failure {
    let code = match &error {
        Revision3VoiceTakeRemovalConflictV1::CurrentHeadMismatch => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_HEAD_CONFLICT"
        }
        Revision3VoiceTakeRemovalConflictV1::ProjectIdentityMismatch { .. }
        | Revision3VoiceTakeRemovalConflictV1::ProjectRevisionConflict { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_PROJECT_CONFLICT"
        }
        Revision3VoiceTakeRemovalConflictV1::ProjectTargetMismatch => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_TARGET_CONFLICT"
        }
        Revision3VoiceTakeRemovalConflictV1::ProjectRevisionOverflow
        | Revision3VoiceTakeRemovalConflictV1::VoiceSlotRevisionOverflow { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REVISION_LIMIT"
        }
        Revision3VoiceTakeRemovalConflictV1::InvalidEntityIdentity
        | Revision3VoiceTakeRemovalConflictV1::InvalidExpectedLocId => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_REJECTED"
        }
        Revision3VoiceTakeRemovalConflictV1::InvalidDialogLine { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_LINE_CONFLICT"
        }
        Revision3VoiceTakeRemovalConflictV1::InvalidLocalizationReference { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_LOCALIZATION_CONFLICT"
        }
        Revision3VoiceTakeRemovalConflictV1::LocalizationIdentityMismatch { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_LOC_ID_CONFLICT"
        }
        Revision3VoiceTakeRemovalConflictV1::VoiceSlotIdentityMismatch { .. }
        | Revision3VoiceTakeRemovalConflictV1::InvalidVoiceSlot { .. }
        | Revision3VoiceTakeRemovalConflictV1::VoiceSlotRevisionConflict { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_SLOT_CONFLICT"
        }
        Revision3VoiceTakeRemovalConflictV1::InvalidVoiceTake { .. }
        | Revision3VoiceTakeRemovalConflictV1::VoiceTakeRevisionConflict { .. }
        | Revision3VoiceTakeRemovalConflictV1::VoiceTakeLocaleMismatch { .. }
        | Revision3VoiceTakeRemovalConflictV1::VoiceTakeNotExactCandidate { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_TAKE_CONFLICT"
        }
        Revision3VoiceTakeRemovalConflictV1::CurrentSelectionMismatch { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_SELECTION_CONFLICT"
        }
        Revision3VoiceTakeRemovalConflictV1::InvalidLocalBacklink { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_BACKLINK_CONFLICT"
        }
        Revision3VoiceTakeRemovalConflictV1::ReferenceLimit { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REFERENCE_LIMIT"
        }
        Revision3VoiceTakeRemovalConflictV1::CandidateTooLarge { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_PROJECT_LIMIT"
        }
        Revision3VoiceTakeRemovalConflictV1::CandidateNotPersistable { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_PROJECT_INVALID"
        }
    };
    Failure::new(code, error.to_string())
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::InvalidLimits(_) => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_STORE_LIMITS_INVALID"
        }
        WorkingStoreError::MissingRoot(_) => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_STORE_ROOT_MISSING"
        }
        WorkingStoreError::UnsafePath { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_STORE_PATH_UNSAFE"
        }
        WorkingStoreError::LimitExceeded { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_STORE_LIMIT"
        }
        WorkingStoreError::HeadConflict { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_HEAD_CONFLICT"
        }
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_STORE_OBJECT_MISSING"
        }
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_STORE_COLLISION"
        }
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_STORE_IO"
        }
    };
    Failure::new(
        code,
        "the revision-3 Voice take removal Store operation failed",
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
        AssetMeta, AssetStoreIndex, ContentSeal, FormatV2, GameGenerationAnchor, ImportedOgg,
        ProjectId, ProjectMeta, Sha256Digest,
    };
    use tempfile::TempDir;

    use super::*;

    const LOC_ID_ONE: &str = "GRD_263_ASGHAN_OPEN_INFO_06_02";
    const LOC_ID_TWO: &str = "STT_302_VIPER_GREET_INFO_11_02";
    const LOCALIZATION_ID_BYTE: u8 = 0x42;
    const LINE_ID_BYTE: u8 = 0x43;
    const SLOT_ID_BYTE: u8 = 0x44;
    const SELECTED_TAKE_ID_BYTE: u8 = 0x45;
    const SHARED_TAKE_ID_BYTE: u8 = 0x46;
    const UNIQUE_TAKE_ID_BYTE: u8 = 0x47;
    const OTHER_LOCALIZATION_ID_BYTE: u8 = 0x48;
    const OTHER_LINE_ID_BYTE: u8 = 0x49;
    const OTHER_SLOT_ID_BYTE: u8 = 0x4a;
    const SELECTED_TAKE_REVISION: u64 = 4;
    const SHARED_TAKE_REVISION: u64 = 5;
    const UNIQUE_TAKE_REVISION: u64 = 6;

    struct PublishedStore {
        temp: TempDir,
        project: ProjectRevision3,
        project_json: String,
        head: WorkingHead,
        fixed_head_bytes: Vec<u8>,
        assets: BTreeMap<Sha256Digest, (PathBuf, Vec<u8>)>,
    }

    fn id(tag: u8) -> EntityId {
        EntityId::from_bytes([tag; 16])
    }

    fn project_id(tag: u8) -> ProjectId {
        ProjectId::from_bytes([tag; 16])
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
            importer: "voice-take-removal-ffi-tests".to_owned(),
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
            project_id: project_id(0x51),
            revision,
            meta: ProjectMeta {
                name: "Voice take removal FFI fixture".to_owned(),
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

    fn take_entity(tag: u8, revision: u64, logical_name: &str, imported: &ImportedOgg) -> Entity {
        let mut asset = imported.asset.clone();
        asset.logical_name = logical_name.to_owned();
        entity(
            tag,
            revision,
            logical_name,
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
                status: VoiceTakeStatus::Approved,
            }),
        )
    }

    fn voice_project(
        revision: u64,
        common: &ImportedOgg,
        unique: &ImportedOgg,
    ) -> ProjectRevision3 {
        let pid = project_id(0x51);
        let take_ref = |take| TypedRef::new(pid, take, EntityKind::VoiceTake);
        let mut project = empty_project(revision);
        for imported in [common, unique] {
            project.asset_store.assets.insert(
                imported.asset.sha256,
                AssetMeta {
                    byte_len: imported.asset.byte_len,
                    media_type: "audio/ogg".to_owned(),
                },
            );
        }
        project.entities = BTreeMap::from([
            (
                id(LOCALIZATION_ID_BYTE),
                entity(
                    LOCALIZATION_ID_BYTE,
                    2,
                    "Asghan line text",
                    EntityPayload::LocalizationEntry(LocalizationEntry {
                        loc_id: LOC_ID_ONE.to_owned(),
                        texts: BTreeMap::new(),
                    }),
                ),
            ),
            (
                id(LINE_ID_BYTE),
                entity(
                    LINE_ID_BYTE,
                    2,
                    "Asghan greeting",
                    EntityPayload::DialogLine(DialogLine {
                        localization: TypedRef::new(
                            pid,
                            id(LOCALIZATION_ID_BYTE),
                            EntityKind::LocalizationEntry,
                        ),
                        speaker_hint: Some("Asghan".to_owned()),
                        voice_slots: BTreeMap::from([(
                            locale(),
                            TypedRef::new(pid, id(SLOT_ID_BYTE), EntityKind::VoiceSlot),
                        )]),
                    }),
                ),
            ),
            (
                id(SLOT_ID_BYTE),
                entity(
                    SLOT_ID_BYTE,
                    3,
                    "Asghan German voice slot",
                    EntityPayload::VoiceSlot(VoiceSlot {
                        locale: locale(),
                        target_resolution: VoiceTargetResolution::Unresolved,
                        candidates: vec![
                            take_ref(id(SELECTED_TAKE_ID_BYTE)),
                            take_ref(id(SHARED_TAKE_ID_BYTE)),
                            take_ref(id(UNIQUE_TAKE_ID_BYTE)),
                        ],
                        selected: Some(take_ref(id(SELECTED_TAKE_ID_BYTE))),
                    }),
                ),
            ),
            (
                id(SELECTED_TAKE_ID_BYTE),
                take_entity(
                    SELECTED_TAKE_ID_BYTE,
                    SELECTED_TAKE_REVISION,
                    "asghan-selected.ogg",
                    common,
                ),
            ),
            (
                id(SHARED_TAKE_ID_BYTE),
                take_entity(
                    SHARED_TAKE_ID_BYTE,
                    SHARED_TAKE_REVISION,
                    "shared.ogg",
                    common,
                ),
            ),
            (
                id(UNIQUE_TAKE_ID_BYTE),
                take_entity(
                    UNIQUE_TAKE_ID_BYTE,
                    UNIQUE_TAKE_REVISION,
                    "unique-opus.ogg",
                    unique,
                ),
            ),
            (
                id(OTHER_LOCALIZATION_ID_BYTE),
                entity(
                    OTHER_LOCALIZATION_ID_BYTE,
                    2,
                    "Viper line text",
                    EntityPayload::LocalizationEntry(LocalizationEntry {
                        loc_id: LOC_ID_TWO.to_owned(),
                        texts: BTreeMap::new(),
                    }),
                ),
            ),
            (
                id(OTHER_LINE_ID_BYTE),
                entity(
                    OTHER_LINE_ID_BYTE,
                    2,
                    "Viper greeting",
                    EntityPayload::DialogLine(DialogLine {
                        localization: TypedRef::new(
                            pid,
                            id(OTHER_LOCALIZATION_ID_BYTE),
                            EntityKind::LocalizationEntry,
                        ),
                        speaker_hint: Some("Viper".to_owned()),
                        voice_slots: BTreeMap::from([(
                            locale(),
                            TypedRef::new(pid, id(OTHER_SLOT_ID_BYTE), EntityKind::VoiceSlot),
                        )]),
                    }),
                ),
            ),
            (
                id(OTHER_SLOT_ID_BYTE),
                entity(
                    OTHER_SLOT_ID_BYTE,
                    4,
                    "Viper German voice slot",
                    EntityPayload::VoiceSlot(VoiceSlot {
                        locale: locale(),
                        target_resolution: VoiceTargetResolution::Unresolved,
                        candidates: vec![take_ref(id(SHARED_TAKE_ID_BYTE))],
                        selected: Some(take_ref(id(SHARED_TAKE_ID_BYTE))),
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

    fn import_ogg(
        store: &WorkingProjectStore,
        source_name: &str,
        bytes: &[u8],
        expected_head: &WorkingHead,
    ) -> ImportedOgg {
        let source_temp = TempDir::new().unwrap();
        let source = source_temp.path().join(source_name);
        fs::write(&source, bytes).unwrap();
        store
            .import_ogg(&source, source_name, Some(expected_head))
            .unwrap()
    }

    fn published_store() -> PublishedStore {
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let empty = empty_project(0);
        let basis = store.prepare_revision3_checkpoint(None, &empty).unwrap();
        fs::write(temp.path().join("gore-project.json"), &basis.head_bytes).unwrap();

        let common = import_ogg(
            &store,
            "common-vorbis.ogg",
            include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
            &basis.head,
        );
        let unique = import_ogg(
            &store,
            "unique-opus.ogg",
            include_bytes!("../../gore-vo/testdata/tiny-opus.ogg"),
            &basis.head,
        );
        let project = voice_project(1, &common, &unique);
        let project_json = project.to_canonical_json().unwrap();
        let published = store
            .prepare_revision3_checkpoint(Some(&basis.head), &project)
            .unwrap();
        fs::write(temp.path().join("gore-project.json"), &published.head_bytes).unwrap();
        let assets = [common.asset.sha256, unique.asset.sha256]
            .into_iter()
            .map(|digest| {
                let path = asset_path(temp.path(), digest);
                (digest, (path.clone(), fs::read(path).unwrap()))
            })
            .collect();
        PublishedStore {
            temp,
            project,
            project_json,
            head: published.head,
            fixed_head_bytes: published.head_bytes,
            assets,
        }
    }

    fn removal_request(
        store: &PublishedStore,
        take_id: EntityId,
    ) -> Revision3VoiceTakeRemovalRequestV1 {
        let slot = slot(&store.project, id(SLOT_ID_BYTE));
        Revision3VoiceTakeRemovalRequestV1 {
            expected_head: store.head.clone(),
            expected_project_id: store.project.project_id,
            expected_revision: store.project.revision,
            expected_target: store.project.target.clone(),
            line_id: id(LINE_ID_BYTE),
            localization_id: id(LOCALIZATION_ID_BYTE),
            expected_loc_id: LOC_ID_ONE.to_owned(),
            locale: locale(),
            slot_id: id(SLOT_ID_BYTE),
            expected_slot_revision: store.project.entities[&id(SLOT_ID_BYTE)].revision,
            take_id,
            expected_take_revision: store.project.entities[&take_id].revision,
            expected_selected_take_id: slot.selected.as_ref().map(|reference| reference.id),
        }
    }

    fn raw_request(payload: Value) -> String {
        serde_json::to_string(&json!({"command": COMMAND, "payload": payload})).unwrap()
    }

    fn wire(root: &Path, project_json: &str, request_json: &str) -> String {
        raw_request(json!({
            "current_project_json": project_json,
            "root": root,
            "voice_take_removal_request_json": request_json,
        }))
    }

    fn call(store: &PublishedStore, request: &Revision3VoiceTakeRemovalRequestV1) -> Value {
        prepare_revision3_voice_take_removal_v1_raw(&wire(
            store.temp.path(),
            &store.project_json,
            &request.to_canonical_json().unwrap(),
        ))
    }

    fn slot(project: &ProjectRevision3, slot_id: EntityId) -> &VoiceSlot {
        let EntityPayload::VoiceSlot(slot) = &project.entities[&slot_id].payload else {
            panic!("expected VoiceSlot")
        };
        slot
    }

    fn slot_mut(project: &mut ProjectRevision3, slot_id: EntityId) -> &mut VoiceSlot {
        let EntityPayload::VoiceSlot(slot) =
            &mut project.entities.get_mut(&slot_id).unwrap().payload
        else {
            panic!("expected VoiceSlot")
        };
        slot
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

    fn assert_physical_assets_unchanged(store: &PublishedStore) {
        for (path, bytes) in store.assets.values() {
            assert_eq!(&fs::read(path).unwrap(), bytes);
        }
    }

    fn reopen_response(store: &PublishedStore, response: &Value) -> ProjectRevision3 {
        WorkingProjectStore::open_existing(store.temp.path(), ffi_store_limits())
            .unwrap()
            .open_revision3_head_bytes(
                response["head_json"].as_str().unwrap().as_bytes(),
                AssetVerification::Full,
            )
            .unwrap()
            .project
    }

    fn republish(store: &mut PublishedStore, mut project: ProjectRevision3) {
        let working =
            WorkingProjectStore::open_existing(store.temp.path(), ffi_store_limits()).unwrap();
        if project.revision == store.project.revision {
            project.revision += 1;
        }
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
    }

    fn republish_root(store: &mut PublishedStore, project: ProjectRevision3) {
        let working =
            WorkingProjectStore::open_existing(store.temp.path(), ffi_store_limits()).unwrap();
        fs::remove_file(store.temp.path().join("gore-project.json")).unwrap();
        let published = working
            .prepare_revision3_checkpoint(None, &project)
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
    fn exact_wire_dispatch_and_capability_reject_extra_authority() {
        let valid = raw_request(json!({
            "current_project_json": "{}",
            "root": "C:/missing",
            "voice_take_removal_request_json": "{}",
        }));
        let parsed: PrepareVoiceTakeRemovalWirePayload = parse_exact_wire(&valid).unwrap();
        assert_eq!(parsed.current_project_json, "{}");

        let cases = [
            format!(
                "{{\"command\":\"{COMMAND}\",\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"root\":\"r\",\"voice_take_removal_request_json\":\"{{}}\"}}}}"
            ),
            format!(
                "{{\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"root\":\"r\",\"root\":\"forged\",\"voice_take_removal_request_json\":\"{{}}\"}}}}"
            ),
            raw_request(json!({
                "current_project_json": "{}", "root": "r",
                "voice_take_removal_request_json": "{}", "game_root": "forged"
            })),
            raw_request(json!({
                "current_project_json": "{}", "root": "r",
                "voice_take_removal_request_json": "{}", "delete_blob": true
            })),
            format!(" {valid}"),
            format!(
                "{{\"payload\":{{\"current_project_json\":\"{{}}\",\"root\":\"C:/missing\",\"voice_take_removal_request_json\":\"{{}}\"}},\"command\":\"{COMMAND}\"}}"
            ),
        ];
        for input in cases {
            assert_eq!(
                prepare_revision3_voice_take_removal_v1_raw(&input)["error"]["code"],
                "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_INVALID",
                "{input}"
            );
        }

        let dispatched: Value = serde_json::from_str(&crate::execute_json(&valid)).unwrap();
        assert_eq!(
            dispatched["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_INVALID"
        );
        let info: Value = serde_json::from_str(&crate::execute_json(
            r#"{"command":"core_info","payload":{"ignored":true}}"#,
        ))
        .unwrap();
        assert!(info["commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|command| command == COMMAND));
    }

    #[test]
    fn unique_last_use_prepares_only_the_exact_delta_and_preserves_orphan_ogg() {
        let store = published_store();
        let basis_asset_store = store.project.asset_store.clone();
        let request = removal_request(&store, id(UNIQUE_TAKE_ID_BYTE));
        let response = call(&store, &request);
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["outcome"], "prepared_unpublished");
        assert_eq!(response["revision"], 2);
        assert_eq!(response["line_id"], id(LINE_ID_BYTE).to_string());
        assert_eq!(
            response["localization_id"],
            id(LOCALIZATION_ID_BYTE).to_string()
        );
        assert_eq!(response["slot_id"], id(SLOT_ID_BYTE).to_string());
        assert_eq!(response["slot_revision"], 4);
        assert_eq!(response["locale"], "de");
        assert_eq!(response["loc_id"], LOC_ID_ONE);
        assert_eq!(response["take_id"], id(UNIQUE_TAKE_ID_BYTE).to_string());
        assert_eq!(response["take_revision"], UNIQUE_TAKE_REVISION);
        assert_eq!(
            response["previous_selected_take_id"],
            id(SELECTED_TAKE_ID_BYTE).to_string()
        );
        assert_eq!(response["selection_cleared"], false);
        assert_eq!(response["take_entity_removed"], true);
        assert_eq!(response["remaining_candidate_count"], 2);
        assert_eq!(response["build_status"], "blocked");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["publication_status"], "not_supported");
        assert_eq!(
            response["basis_head_json"],
            serde_json::to_string(&store.head).unwrap()
        );

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
                "previous_selected_take_id",
                "project_id",
                "project_json",
                "publication_status",
                "remaining_candidate_count",
                "revision",
                "runtime_status",
                "selection_cleared",
                "slot_id",
                "slot_revision",
                "take_entity_removed",
                "take_id",
                "take_revision",
            ])
        );

        let reopened = reopen_response(&store, &response);
        let mut expected = store.project.clone();
        expected.revision += 1;
        expected
            .entities
            .get_mut(&id(SLOT_ID_BYTE))
            .unwrap()
            .revision += 1;
        slot_mut(&mut expected, id(SLOT_ID_BYTE))
            .candidates
            .retain(|candidate| candidate.id != id(UNIQUE_TAKE_ID_BYTE));
        expected.entities.remove(&id(UNIQUE_TAKE_ID_BYTE));
        assert_eq!(reopened, expected);
        assert_eq!(reopened.asset_store, basis_asset_store);
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
        assert_physical_assets_unchanged(&store);
        assert_eq!(
            reopened.to_canonical_json().unwrap(),
            response["project_json"]
        );
        let encoded = response.to_string();
        assert!(!encoded.contains(store.temp.path().to_string_lossy().as_ref()));
        assert!(!encoded.contains("game_root"));
        assert!(!encoded.contains("delete_blob"));
        assert!(!encoded.contains("deploy"));
    }

    #[test]
    fn selected_removal_atomically_clears_only_its_slot_selection() {
        let store = published_store();
        let other_slot_before = store.project.entities[&id(OTHER_SLOT_ID_BYTE)].clone();
        let response = call(&store, &removal_request(&store, id(SELECTED_TAKE_ID_BYTE)));
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["selection_cleared"], true);
        assert_eq!(response["take_entity_removed"], true);
        assert_eq!(response["remaining_candidate_count"], 2);

        let reopened = reopen_response(&store, &response);
        assert!(slot(&reopened, id(SLOT_ID_BYTE)).selected.is_none());
        assert!(!reopened.entities.contains_key(&id(SELECTED_TAKE_ID_BYTE)));
        assert_eq!(
            reopened.entities[&id(OTHER_SLOT_ID_BYTE)],
            other_slot_before
        );
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
        assert_physical_assets_unchanged(&store);
    }

    #[test]
    fn shared_take_is_unlinked_here_but_retained_when_selected_elsewhere() {
        let store = published_store();
        let shared_before = store.project.entities[&id(SHARED_TAKE_ID_BYTE)].clone();
        let other_slot_before = store.project.entities[&id(OTHER_SLOT_ID_BYTE)].clone();
        let response = call(&store, &removal_request(&store, id(SHARED_TAKE_ID_BYTE)));
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["selection_cleared"], false);
        assert_eq!(response["take_entity_removed"], false);
        let reopened = reopen_response(&store, &response);
        assert_eq!(reopened.entities[&id(SHARED_TAKE_ID_BYTE)], shared_before);
        assert_eq!(
            reopened.entities[&id(OTHER_SLOT_ID_BYTE)],
            other_slot_before
        );
        assert_eq!(
            slot(&reopened, id(OTHER_SLOT_ID_BYTE))
                .selected
                .as_ref()
                .unwrap()
                .id,
            id(SHARED_TAKE_ID_BYTE)
        );
        assert!(!slot(&reopened, id(SLOT_ID_BYTE))
            .candidates
            .iter()
            .any(|candidate| candidate.id == id(SHARED_TAKE_ID_BYTE)));
        assert_physical_assets_unchanged(&store);
    }

    #[test]
    fn selected_shared_take_clears_here_but_remains_byte_exact_for_other_slot() {
        let mut store = published_store();
        let mut project = store.project.clone();
        let project_id = project.project_id;
        slot_mut(&mut project, id(SLOT_ID_BYTE)).selected = Some(TypedRef::new(
            project_id,
            id(SHARED_TAKE_ID_BYTE),
            EntityKind::VoiceTake,
        ));
        republish(&mut store, project);

        let asset_store_before = store.project.asset_store.clone();
        let shared_before = store.project.entities[&id(SHARED_TAKE_ID_BYTE)].clone();
        let other_slot_before = store.project.entities[&id(OTHER_SLOT_ID_BYTE)].clone();
        let response = call(&store, &removal_request(&store, id(SHARED_TAKE_ID_BYTE)));
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["selection_cleared"], true);
        assert_eq!(response["take_entity_removed"], false);
        assert_eq!(response["remaining_candidate_count"], 2);

        let reopened = reopen_response(&store, &response);
        assert!(slot(&reopened, id(SLOT_ID_BYTE)).selected.is_none());
        assert!(!slot(&reopened, id(SLOT_ID_BYTE))
            .candidates
            .iter()
            .any(|candidate| candidate.id == id(SHARED_TAKE_ID_BYTE)));
        assert_eq!(reopened.entities[&id(SHARED_TAKE_ID_BYTE)], shared_before);
        assert_eq!(
            reopened.entities[&id(OTHER_SLOT_ID_BYTE)],
            other_slot_before
        );
        assert_eq!(reopened.asset_store, asset_store_before);
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
        assert_physical_assets_unchanged(&store);
    }

    #[test]
    fn sole_final_use_candidate_leaves_an_empty_slot_and_preserves_orphan_ogg() {
        let mut store = published_store();
        let mut project = store.project.clone();
        let project_id = project.project_id;
        let only_take = TypedRef::new(project_id, id(SELECTED_TAKE_ID_BYTE), EntityKind::VoiceTake);
        let target_slot = slot_mut(&mut project, id(SLOT_ID_BYTE));
        target_slot.candidates = vec![only_take.clone()];
        target_slot.selected = Some(only_take);
        republish(&mut store, project);

        let asset_store_before = store.project.asset_store.clone();
        let slot_before = store.project.entities[&id(SLOT_ID_BYTE)].clone();
        let target_before = slot(&store.project, id(SLOT_ID_BYTE))
            .target_resolution
            .clone();
        let response = call(&store, &removal_request(&store, id(SELECTED_TAKE_ID_BYTE)));
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["selection_cleared"], true);
        assert_eq!(response["take_entity_removed"], true);
        assert_eq!(response["remaining_candidate_count"], 0);

        let reopened = reopen_response(&store, &response);
        let reopened_slot_entity = &reopened.entities[&id(SLOT_ID_BYTE)];
        assert!(slot(&reopened, id(SLOT_ID_BYTE)).candidates.is_empty());
        assert!(slot(&reopened, id(SLOT_ID_BYTE)).selected.is_none());
        assert_eq!(
            slot(&reopened, id(SLOT_ID_BYTE)).target_resolution,
            target_before
        );
        assert_eq!(reopened_slot_entity.origin, slot_before.origin);
        assert_eq!(reopened_slot_entity.display_name, slot_before.display_name);
        assert!(!reopened.entities.contains_key(&id(SELECTED_TAKE_ID_BYTE)));
        assert_eq!(reopened.asset_store, asset_store_before);
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
        assert_physical_assets_unchanged(&store);
    }

    #[test]
    fn backlink_preflight_blocks_local_unexpected_edges_but_ignores_foreign_same_id() {
        let mut local = published_store();
        let mut project = local.project.clone();
        project
            .entities
            .get_mut(&id(SELECTED_TAKE_ID_BYTE))
            .unwrap()
            .origin = OriginRef::Generated {
            generator_id: "unsafe-local-owner".to_owned(),
            generator_version: 1,
            owner: TypedRef::new(
                project.project_id,
                id(UNIQUE_TAKE_ID_BYTE),
                EntityKind::VoiceTake,
            ),
        };
        republish(&mut local, project);
        let before = snapshot_regular_files(local.temp.path());
        let response = call(&local, &removal_request(&local, id(UNIQUE_TAKE_ID_BYTE)));
        assert_eq!(
            response["error"]["code"], "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_BACKLINK_CONFLICT",
            "{response}"
        );
        assert_eq!(snapshot_regular_files(local.temp.path()), before);

        let mut kind_mismatch = published_store();
        let mut project = kind_mismatch.project.clone();
        project
            .entities
            .get_mut(&id(SELECTED_TAKE_ID_BYTE))
            .unwrap()
            .origin = OriginRef::Generated {
            generator_id: "unsafe-kind-mismatch".to_owned(),
            generator_version: 1,
            owner: TypedRef::new(
                project.project_id,
                id(UNIQUE_TAKE_ID_BYTE),
                EntityKind::LocalizationEntry,
            ),
        };
        republish(&mut kind_mismatch, project);
        let response = call(
            &kind_mismatch,
            &removal_request(&kind_mismatch, id(UNIQUE_TAKE_ID_BYTE)),
        );
        assert_eq!(
            response["error"]["code"], "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_BACKLINK_CONFLICT",
            "{response}"
        );

        let mut foreign = published_store();
        let mut project = foreign.project.clone();
        project
            .entities
            .get_mut(&id(SELECTED_TAKE_ID_BYTE))
            .unwrap()
            .origin = OriginRef::Generated {
            generator_id: "safe-foreign-owner".to_owned(),
            generator_version: 1,
            owner: TypedRef::new(
                project_id(0x99),
                id(UNIQUE_TAKE_ID_BYTE),
                EntityKind::VoiceTake,
            ),
        };
        republish(&mut foreign, project);
        let response = call(
            &foreign,
            &removal_request(&foreign, id(UNIQUE_TAKE_ID_BYTE)),
        );
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["take_entity_removed"], true);
    }

    #[test]
    fn current_project_and_exact_graph_mismatches_write_no_candidate_objects() {
        let store = published_store();
        let before = snapshot_regular_files(store.temp.path());
        let base = removal_request(&store, id(UNIQUE_TAKE_ID_BYTE));
        let response = prepare_revision3_voice_take_removal_v1_raw(&wire(
            store.temp.path(),
            "not-the-exact-current-project",
            &base.to_canonical_json().unwrap(),
        ));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_PROJECT_CONFLICT"
        );

        let mut cases = Vec::new();
        let mut stale_head = base.clone();
        stale_head.expected_head.snapshot.byte_len += 1;
        cases.push((
            stale_head,
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_HEAD_CONFLICT",
        ));
        let mut foreign_project = base.clone();
        foreign_project.expected_project_id = project_id(0x71);
        cases.push((
            foreign_project,
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_PROJECT_CONFLICT",
        ));
        let mut stale_slot = base.clone();
        stale_slot.expected_slot_revision -= 1;
        cases.push((
            stale_slot,
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_SLOT_CONFLICT",
        ));
        let mut stale_take = base.clone();
        stale_take.expected_take_revision -= 1;
        cases.push((
            stale_take,
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_TAKE_CONFLICT",
        ));
        let mut stale_selection = base.clone();
        stale_selection.expected_selected_take_id = None;
        cases.push((
            stale_selection,
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_SELECTION_CONFLICT",
        ));
        let mut invalid_loc = base;
        invalid_loc.expected_loc_id = "C:/secret/voice".to_owned();
        cases.push((
            invalid_loc,
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_REJECTED",
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
        let mut signed = removal_request(&store, id(UNIQUE_TAKE_ID_BYTE));
        signed.expected_revision = u64::MAX;
        assert_eq!(
            call(&store, &signed)["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_SIGNED_WIRE_LIMIT"
        );
        assert_eq!(snapshot_regular_files(store.temp.path()), before);

        let mut store = published_store();
        let mut project = store.project.clone();
        project.revision = MAX_INCREMENTABLE_REVISION + 1;
        republish_root(&mut store, project);
        let before = snapshot_regular_files(store.temp.path());
        let response = call(&store, &removal_request(&store, id(UNIQUE_TAKE_ID_BYTE)));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REVISION_LIMIT"
        );
        assert_eq!(snapshot_regular_files(store.temp.path()), before);

        let mut store = published_store();
        let mut project = store.project.clone();
        project
            .entities
            .get_mut(&id(SLOT_ID_BYTE))
            .unwrap()
            .revision = MAX_INCREMENTABLE_REVISION + 1;
        republish(&mut store, project);
        let before = snapshot_regular_files(store.temp.path());
        let response = call(&store, &removal_request(&store, id(UNIQUE_TAKE_ID_BYTE)));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REVISION_LIMIT"
        );
        assert_eq!(snapshot_regular_files(store.temp.path()), before);
    }

    #[test]
    fn corrupted_store_is_path_free_and_never_publishes_a_candidate() {
        let store = published_store();
        let request = removal_request(&store, id(UNIQUE_TAKE_ID_BYTE));
        let unique_digest = match &store.project.entities[&id(UNIQUE_TAKE_ID_BYTE)].payload {
            EntityPayload::VoiceTake(take) => take.asset.sha256,
            _ => unreachable!(),
        };
        fs::remove_file(&store.assets[&unique_digest].0).unwrap();
        let response = call(&store, &request);
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_STORE_OBJECT_MISSING",
            "{response}"
        );
        assert!(!response
            .to_string()
            .contains(store.temp.path().to_string_lossy().as_ref()));
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
    }

    #[test]
    fn all_three_head_drift_gates_preserve_the_external_publisher() {
        for gate in ["pre", "post", "final"] {
            let store = published_store();
            let rival = rival_head(&store, &format!("External {gate} publisher"));
            let request = removal_request(&store, id(UNIQUE_TAKE_ID_BYTE));
            let wire = wire(
                store.temp.path(),
                &store.project_json,
                &request.to_canonical_json().unwrap(),
            );
            let publish = || {
                fs::write(store.temp.path().join("gore-project.json"), &rival).unwrap();
            };
            let result = match gate {
                "pre" => prepare_revision3_voice_take_removal_v1_inner_with_test_seams(
                    &wire,
                    publish,
                    || {},
                    || {},
                ),
                "post" => prepare_revision3_voice_take_removal_v1_inner_with_test_seams(
                    &wire,
                    || {},
                    publish,
                    || {},
                ),
                "final" => prepare_revision3_voice_take_removal_v1_inner_with_test_seams(
                    &wire,
                    || {},
                    || {},
                    publish,
                ),
                _ => unreachable!(),
            };
            let response = result.unwrap_err().response();
            assert_eq!(
                response["error"]["code"], "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_HEAD_CONFLICT",
                "{gate}: {response}"
            );
            assert_eq!(fixed_head(&store), rival);
        }
    }

    #[test]
    fn nested_request_is_canonical_bounded_and_cannot_smuggle_authority() {
        let store = published_store();
        let before = snapshot_regular_files(store.temp.path());
        let canonical = removal_request(&store, id(UNIQUE_TAKE_ID_BYTE))
            .to_canonical_json()
            .unwrap();
        for nested in [
            "{}".to_owned(),
            format!(" {canonical}"),
            canonical.replacen(
                "\"expected_revision\":1",
                "\"expected_revision\":1,\"expected_revision\":1",
                1,
            ),
            canonical.replacen(
                "\"expected_selected_take_id\"",
                "\"delete_physical_ogg\":true,\"expected_selected_take_id\"",
                1,
            ),
        ] {
            let response = prepare_revision3_voice_take_removal_v1_raw(&wire(
                store.temp.path(),
                &store.project_json,
                &nested,
            ));
            assert_eq!(
                response["error"]["code"], "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_INVALID",
                "{response}"
            );
        }
        let oversized = "x".repeat(MAX_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_JSON_BYTES_V1 + 1);
        assert_eq!(
            prepare_revision3_voice_take_removal_v1_raw(&wire(
                store.temp.path(),
                &store.project_json,
                &oversized,
            ))["error"]["code"],
            "AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_LIMIT"
        );
        assert_eq!(snapshot_regular_files(store.temp.path()), before);
    }
}
