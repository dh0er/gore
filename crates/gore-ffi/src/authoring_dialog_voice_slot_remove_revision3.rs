//! Prepare-only removal of one exact empty revision-3 DialogLine VoiceSlot relationship.
//!
//! This route accepts no game, save, media, build, deployment, runtime, or fixed-head publication
//! authority. It fully opens the exact current Store project, binds the complete line/localization/
//! locale/slot graph, evaluates the pure transaction, independently reconstructs the permitted
//! delta, and fully reopens an immutable candidate before returning it.

use std::path::Path;

use gore_authoring::model_revision3::{
    EntityKind, EntityPayload, OriginRef, VoiceTargetResolution,
};
use gore_authoring::{
    apply_revision3_dialog_voice_slot_removal_transaction_v1, build_revision3_content_index_v1,
    validate_revision3_voice_loc_id_basename_stem_v1, AssetVerification, EntityId, LocaleCode,
    ProjectRevision3, Revision3ContentReferenceResolutionV1, Revision3ContentReferenceRoleV1,
    Revision3DialogVoiceSlotRemovalBuildStatusV1, Revision3DialogVoiceSlotRemovalConflictV1,
    Revision3DialogVoiceSlotRemovalErrorV1, Revision3DialogVoiceSlotRemovalEvaluationV1,
    Revision3DialogVoiceSlotRemovalOutcomeV1, Revision3DialogVoiceSlotRemovalPublicationStatusV1,
    Revision3DialogVoiceSlotRemovalRequestV1, Revision3DialogVoiceSlotRemovalRuntimeStatusV1,
    Revision3DialogVoiceSlotRemovalTargetAuthorityV1,
    Revision3DialogVoiceSlotRemovalTargetResolutionV1, WorkingHead, WorkingProjectStore,
    WorkingStoreError, WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
    MAX_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_JSON_BYTES_V1,
    REVISION3_VOICE_SLOT_GENERATOR_ID_V1, REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_prepare_revision3_dialog_voice_slot_removal_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_INCREMENTABLE_REVISION: u64 = i64::MAX as u64 - 1;
const MAX_WIRE_REVISION: u64 = i64::MAX as u64;
const MAX_WIRE_BYTES: usize = MAX_PROJECT_JSON_BYTES * 2
    + MAX_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_JSON_BYTES_V1 * 2
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
struct PrepareDialogVoiceSlotRemovalWirePayload {
    current_project_json: String,
    root: String,
    dialog_voice_slot_removal_request_json: String,
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
struct BoundDialogVoiceSlotRemoval {
    line_revision: u64,
    slot_revision: u64,
    removed_target_resolution: Revision3DialogVoiceSlotRemovalTargetResolutionV1,
}

pub(super) fn prepare_revision3_dialog_voice_slot_removal_v1_raw(input: &str) -> Value {
    prepare_revision3_dialog_voice_slot_removal_v1_inner(input).unwrap_or_else(Failure::response)
}

fn prepare_revision3_dialog_voice_slot_removal_v1_inner(input: &str) -> Result<Value, Failure> {
    prepare_revision3_dialog_voice_slot_removal_v1_inner_with_test_seams(input, || {}, || {}, || {})
}

fn prepare_revision3_dialog_voice_slot_removal_v1_inner_with_test_seams<B, A, F>(
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
    let payload: PrepareDialogVoiceSlotRemovalWirePayload = parse_exact_wire(input)?;
    validate_payload(&payload)?;

    let request = Revision3DialogVoiceSlotRemovalRequestV1::from_json(
        &payload.dialog_voice_slot_removal_request_json,
    )
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
    let outcome = match apply_revision3_dialog_voice_slot_removal_transaction_v1(
        &basis.head,
        &canonical_basis,
        &payload.dialog_voice_slot_removal_request_json,
    )
    .map_err(map_transaction_error)?
    {
        Revision3DialogVoiceSlotRemovalEvaluationV1::Applied(outcome) => *outcome,
        Revision3DialogVoiceSlotRemovalEvaluationV1::Rejected(rejection) => {
            return Err(map_transaction_conflict(rejection.conflict));
        }
    };

    require_signed_serializable(&outcome.project)?;
    verify_outcome_binding(&basis.head, &basis.project, &request, bound, &outcome)?;
    match outcome.build_status {
        Revision3DialogVoiceSlotRemovalBuildStatusV1::Blocked => {}
    }
    match outcome.runtime_status {
        Revision3DialogVoiceSlotRemovalRuntimeStatusV1::RuntimeUnqualified => {}
    }
    match outcome.target_authority {
        Revision3DialogVoiceSlotRemovalTargetAuthorityV1::NotGranted => {}
    }
    match outcome.publication_status {
        Revision3DialogVoiceSlotRemovalPublicationStatusV1::NotSupported => {}
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
            "the prepared dialog VoiceSlot removal checkpoint did not fully reopen exactly",
        ));
    }
    let reopened_json = reopened.project.to_canonical_json().map_err(|_| {
        invariant("the fully reopened dialog VoiceSlot removal candidate could not be serialized")
    })?;
    if reopened_json != outcome.canonical_project_json {
        return Err(invariant(
            "the fully reopened dialog VoiceSlot removal candidate changed canonical bytes",
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
        "line_revision": outcome.line_revision,
        "localization_id": outcome.localization_id.to_string(),
        "slot_id": outcome.slot_id.to_string(),
        "removed_slot_revision": outcome.removed_slot_revision,
        "locale": outcome.locale.to_string(),
        "loc_id": outcome.loc_id,
        "removed_target_resolution": target_resolution_wire(outcome.removed_target_resolution),
        "build_status": "blocked",
        "runtime_status": "runtime_unqualified",
        "target_authority": "not_granted",
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
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_INPUT_LIMIT",
            format!(
                "revision-3 dialog VoiceSlot removal request exceeds the {MAX_WIRE_BYTES}-byte wire limit"
            ),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    let canonical = serde_json::to_string(&request).map_err(|_| {
        invariant("the dialog VoiceSlot removal wire request could not be serialized")
    })?;
    if canonical.as_bytes() != input.as_bytes() {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_payload(payload: &PrepareDialogVoiceSlotRemovalWirePayload) -> Result<(), Failure> {
    validate_path(&payload.root)?;
    if payload.current_project_json.is_empty()
        || payload.dialog_voice_slot_removal_request_json.is_empty()
    {
        return Err(invalid_request());
    }
    if payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_PROJECT_LIMIT",
            format!("current_project_json exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ));
    }
    if payload.dialog_voice_slot_removal_request_json.len()
        > MAX_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_JSON_BYTES_V1
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_LIMIT",
            format!(
                "dialog_voice_slot_removal_request_json exceeds the {MAX_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_JSON_BYTES_V1}-byte limit"
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

fn validate_request_shape(
    request: &Revision3DialogVoiceSlotRemovalRequestV1,
) -> Result<(), Failure> {
    let identities = [request.line_id, request.localization_id, request.slot_id];
    if identities.iter().copied().any(is_zero_entity_id)
        || identities
            .iter()
            .enumerate()
            .any(|(index, id)| identities[index + 1..].contains(id))
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_REJECTED",
            "line, localization, and VoiceSlot identities must be non-zero and distinct",
        ));
    }
    if validate_revision3_voice_loc_id_basename_stem_v1(&request.expected_loc_id).is_err() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_REJECTED",
            "expected_loc_id is not one bounded localization identity",
        ));
    }
    Ok(())
}

fn validate_basis_revisions(
    project: &ProjectRevision3,
    request: &Revision3DialogVoiceSlotRemovalRequestV1,
) -> Result<(), Failure> {
    if project.revision > MAX_INCREMENTABLE_REVISION {
        return Err(revision_limit(
            "the published project revision cannot be incremented on the signed wire",
        ));
    }
    if let Some(line) = project.entities.get(&request.line_id) {
        if line.revision > MAX_INCREMENTABLE_REVISION {
            return Err(revision_limit(
                "the published DialogLine revision cannot be incremented on the signed wire",
            ));
        }
    }
    if let Some(slot) = project.entities.get(&request.slot_id) {
        if slot.revision > MAX_WIRE_REVISION {
            return Err(revision_limit(
                "the published VoiceSlot revision exceeds the signed wire",
            ));
        }
    }
    Ok(())
}

fn bind_request_to_basis(
    head: &WorkingHead,
    project: &ProjectRevision3,
    request: &Revision3DialogVoiceSlotRemovalRequestV1,
) -> Result<BoundDialogVoiceSlotRemoval, Failure> {
    if request.expected_head != *head {
        return Err(head_conflict());
    }
    if request.expected_project_id != project.project_id
        || request.expected_revision != project.revision
    {
        return Err(project_conflict(
            "the dialog VoiceSlot removal request project differs from the exact published project",
        ));
    }
    if request.expected_target != project.target {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_TARGET_CONFLICT",
            "the dialog VoiceSlot removal request target differs from the exact published target",
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
    if line_entity.revision != request.expected_line_revision {
        return Err(line_conflict(
            "the DialogLine revision differs from the exact published entity revision",
        ));
    }
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
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_LOC_ID_CONFLICT",
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
    if !slot.candidates.is_empty() || slot.selected.is_some() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_NOT_EMPTY",
            "the requested VoiceSlot still retains a VoiceTake candidate or selection",
        ));
    }
    if !matches!(
        &slot_entity.origin,
        OriginRef::Generated {
            generator_id,
            generator_version,
            owner,
        } if generator_id == REVISION3_VOICE_SLOT_GENERATOR_ID_V1
            && *generator_version == REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1
            && owner.project_id == project.project_id
            && owner.id == request.line_id
            && owner.expected_kind == EntityKind::DialogLine
    ) {
        return Err(slot_conflict(
            "the requested VoiceSlot is not the exact managed slot generated for this DialogLine",
        ));
    }
    validate_slot_backlinks(project, request)?;

    Ok(BoundDialogVoiceSlotRemoval {
        line_revision: line_entity.revision,
        slot_revision: slot_entity.revision,
        removed_target_resolution: target_resolution(&slot.target_resolution),
    })
}

fn validate_slot_backlinks(
    project: &ProjectRevision3,
    request: &Revision3DialogVoiceSlotRemovalRequestV1,
) -> Result<(), Failure> {
    let index = build_revision3_content_index_v1(project).map_err(|error| {
        Failure::new(
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REFERENCE_LIMIT",
            format!("the exact dialog VoiceSlot graph cannot be indexed safely: {error}"),
        )
    })?;
    let mut exact_owner_edges = 0usize;
    for source in &index.entities {
        for reference in &source.references {
            if reference.target.project_id != project.project_id
                || reference.target.entity_id != request.slot_id
            {
                continue;
            }
            let exact_owner = source.id == request.line_id
                && source.kind == EntityKind::DialogLine
                && reference.role == Revision3ContentReferenceRoleV1::DialogVoiceSlot
                && reference.qualifier.as_deref() == Some(request.locale.as_str())
                && reference.target.expected_kind == EntityKind::VoiceSlot
                && reference.resolution == Revision3ContentReferenceResolutionV1::Resolved;
            if !exact_owner {
                return Err(Failure::new(
                    "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_BACKLINK_CONFLICT",
                    "the VoiceSlot has an unresolved, kind-mismatched, qualified, or unexpected same-project backlink",
                ));
            }
            exact_owner_edges += 1;
        }
    }
    if exact_owner_edges != 1 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_BACKLINK_CONFLICT",
            "the VoiceSlot does not have exactly one resolved local DialogLine/locale backlink",
        ));
    }
    Ok(())
}

fn verify_outcome_binding(
    basis_head: &WorkingHead,
    basis: &ProjectRevision3,
    request: &Revision3DialogVoiceSlotRemovalRequestV1,
    bound: BoundDialogVoiceSlotRemoval,
    outcome: &Revision3DialogVoiceSlotRemovalOutcomeV1,
) -> Result<(), Failure> {
    let expected_project_revision = basis
        .revision
        .checked_add(1)
        .ok_or_else(|| revision_limit("the project revision cannot be incremented"))?;
    let expected_line_revision = bound
        .line_revision
        .checked_add(1)
        .ok_or_else(|| revision_limit("the DialogLine revision cannot be incremented"))?;
    if outcome.basis_head != *basis_head
        || outcome.line_id != request.line_id
        || outcome.line_revision != expected_line_revision
        || outcome.localization_id != request.localization_id
        || outcome.slot_id != request.slot_id
        || outcome.removed_slot_revision != bound.slot_revision
        || outcome.locale != request.locale
        || outcome.loc_id != request.expected_loc_id
        || outcome.removed_target_resolution != bound.removed_target_resolution
        || outcome.project.project_id != basis.project_id
        || outcome.project.target != basis.target
        || outcome.project.revision != expected_project_revision
    {
        return Err(invariant(
            "the dialog VoiceSlot removal transaction changed its exact project/request binding",
        ));
    }

    // Reconstruct the complete allowed delta from the fully verified basis: project and line
    // revisions advance, exactly one locale edge is removed, and exactly that empty slot entity
    // disappears. Localization, target evidence outside the removed slot, assets, and every other
    // entity remain byte-equivalent.
    let mut expected = basis.clone();
    expected.revision = expected_project_revision;
    let line_entity = expected.entities.get_mut(&request.line_id).ok_or_else(|| {
        invariant("the bound DialogLine disappeared while reconstructing VoiceSlot removal")
    })?;
    line_entity.revision = expected_line_revision;
    let EntityPayload::DialogLine(line) = &mut line_entity.payload else {
        return Err(invariant(
            "the bound DialogLine changed kind while reconstructing VoiceSlot removal",
        ));
    };
    let removed_reference = line.voice_slots.remove(&request.locale).ok_or_else(|| {
        invariant("the bound DialogLine lost its VoiceSlot edge before reconstruction")
    })?;
    if removed_reference.project_id != basis.project_id
        || removed_reference.expected_kind != EntityKind::VoiceSlot
        || removed_reference.id != request.slot_id
    {
        return Err(invariant(
            "the independently removed DialogLine edge was not the bound VoiceSlot",
        ));
    }
    let removed_entity = expected.entities.remove(&request.slot_id).ok_or_else(|| {
        invariant("the bound VoiceSlot disappeared before independent entity removal")
    })?;
    let EntityPayload::VoiceSlot(removed_slot) = &removed_entity.payload else {
        return Err(invariant(
            "the independently removed entity was not the bound VoiceSlot",
        ));
    };
    if removed_entity.revision != bound.slot_revision
        || removed_slot.locale != request.locale
        || !removed_slot.candidates.is_empty()
        || removed_slot.selected.is_some()
        || target_resolution(&removed_slot.target_resolution) != bound.removed_target_resolution
    {
        return Err(invariant(
            "the independently removed VoiceSlot changed its bound facts",
        ));
    }
    let expected_json = expected.to_canonical_json().map_err(|_| {
        invariant("the independently reconstructed dialog VoiceSlot removal candidate is invalid")
    })?;
    if outcome.project != expected || outcome.canonical_project_json != expected_json {
        return Err(invariant(
            "the dialog VoiceSlot removal transaction changed content outside its exact closure",
        ));
    }
    Ok(())
}

fn target_resolution(
    value: &VoiceTargetResolution,
) -> Revision3DialogVoiceSlotRemovalTargetResolutionV1 {
    match value {
        VoiceTargetResolution::Unresolved => {
            Revision3DialogVoiceSlotRemovalTargetResolutionV1::Unresolved
        }
        VoiceTargetResolution::Ambiguous { .. } => {
            Revision3DialogVoiceSlotRemovalTargetResolutionV1::Ambiguous
        }
        VoiceTargetResolution::Resolved { .. } => {
            Revision3DialogVoiceSlotRemovalTargetResolutionV1::Resolved
        }
    }
}

fn target_resolution_wire(
    value: Revision3DialogVoiceSlotRemovalTargetResolutionV1,
) -> &'static str {
    match value {
        Revision3DialogVoiceSlotRemovalTargetResolutionV1::Unresolved => "unresolved",
        Revision3DialogVoiceSlotRemovalTargetResolutionV1::Ambiguous => "ambiguous",
        Revision3DialogVoiceSlotRemovalTargetResolutionV1::Resolved => "resolved",
    }
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
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_RESPONSE_LIMIT",
            "the prepared dialog VoiceSlot removal head exceeds its bounded transport limit",
        ));
    }
    let value = std::str::from_utf8(head_bytes).map_err(|_| {
        invariant("the prepared dialog VoiceSlot removal head is not canonical UTF-8 JSON")
    })?;
    let parsed: WorkingHead = serde_json::from_str(value)
        .map_err(|_| invariant("the prepared dialog VoiceSlot removal head is invalid JSON"))?;
    let canonical = canonical_head_json(&parsed)?;
    if canonical != value {
        return Err(invariant(
            "the prepared dialog VoiceSlot removal head is not in canonical spelling",
        ));
    }
    Ok(canonical)
}

fn canonical_head_json(head: &WorkingHead) -> Result<String, Failure> {
    require_signed_serializable(head)?;
    let value = serde_json::to_string(head)
        .map_err(|_| invariant("the dialog VoiceSlot removal head could not be serialized"))?;
    if value.is_empty() || value.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_RESPONSE_LIMIT",
            "the dialog VoiceSlot removal head exceeds its bounded transport limit",
        ));
    }
    Ok(value)
}

fn require_signed_serializable(value: &impl Serialize) -> Result<(), Failure> {
    let value = serde_json::to_value(value)
        .map_err(|_| invariant("a dialog VoiceSlot removal wire value could not be inspected"))?;
    require_signed_json_value(&value)
}

fn require_signed_json_value(value: &Value) -> Result<(), Failure> {
    match value {
        Value::Number(number) if number.as_u64().is_some_and(|value| value > i64::MAX as u64) => {
            Err(Failure::new(
                "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_SIGNED_WIRE_LIMIT",
                "a dialog VoiceSlot removal wire integer exceeds the signed 64-bit transport range",
            ))
        }
        Value::Array(values) => values.iter().try_for_each(require_signed_json_value),
        Value::Object(values) => values.values().try_for_each(require_signed_json_value),
        _ => Ok(()),
    }
}

fn enforce_response_budget(response: &Value) -> Result<(), Failure> {
    let encoded = serde_json::to_vec(response)
        .map_err(|_| invariant("the dialog VoiceSlot removal response could not be serialized"))?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_RESPONSE_LIMIT",
            "the dialog VoiceSlot removal response exceeds its bounded transport budget",
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
        "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and exactly current_project_json, root, and dialog_voice_slot_removal_request_json",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_HEAD_CONFLICT",
        "the published revision-3 head changed or differs from the dialog VoiceSlot removal request",
    )
}

fn project_conflict(message: impl Into<String>) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_PROJECT_CONFLICT",
        message,
    )
}

fn line_conflict(message: impl Into<String>) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_LINE_CONFLICT",
        message,
    )
}

fn localization_conflict(message: impl Into<String>) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_LOCALIZATION_CONFLICT",
        message,
    )
}

fn slot_conflict(message: impl Into<String>) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_SLOT_CONFLICT",
        message,
    )
}

fn revision_limit(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REVISION_LIMIT",
        message,
    )
}

fn invariant(message: impl Into<String>) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_INVARIANT",
        message,
    )
}

fn map_request_error(error: impl std::fmt::Display) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_INVALID",
        format!("the exact dialog VoiceSlot removal request is invalid: {error}"),
    )
}

fn map_transaction_error(error: Revision3DialogVoiceSlotRemovalErrorV1) -> Failure {
    match error {
        Revision3DialogVoiceSlotRemovalErrorV1::InvalidProject(error) => Failure::new(
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_PROJECT_INVALID",
            format!("the exact current project is invalid: {error}"),
        ),
        Revision3DialogVoiceSlotRemovalErrorV1::InvalidRequest(error) => map_request_error(error),
        Revision3DialogVoiceSlotRemovalErrorV1::ContentIndex(error) => Failure::new(
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_PROJECT_INVALID",
            format!("the exact project content index is invalid: {error}"),
        ),
        Revision3DialogVoiceSlotRemovalErrorV1::ReopenCandidate(_)
        | Revision3DialogVoiceSlotRemovalErrorV1::CanonicalReopenMismatch
        | Revision3DialogVoiceSlotRemovalErrorV1::CandidatePreservationMismatch => {
            invariant("the pure dialog VoiceSlot removal candidate failed canonical preservation")
        }
    }
}

fn map_transaction_conflict(error: Revision3DialogVoiceSlotRemovalConflictV1) -> Failure {
    let code = match &error {
        Revision3DialogVoiceSlotRemovalConflictV1::CurrentHeadMismatch => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_HEAD_CONFLICT"
        }
        Revision3DialogVoiceSlotRemovalConflictV1::ProjectIdentityMismatch { .. }
        | Revision3DialogVoiceSlotRemovalConflictV1::ProjectRevisionConflict { .. } => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_PROJECT_CONFLICT"
        }
        Revision3DialogVoiceSlotRemovalConflictV1::ProjectTargetMismatch => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_TARGET_CONFLICT"
        }
        Revision3DialogVoiceSlotRemovalConflictV1::ProjectRevisionOverflow
        | Revision3DialogVoiceSlotRemovalConflictV1::DialogLineRevisionOverflow { .. } => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REVISION_LIMIT"
        }
        Revision3DialogVoiceSlotRemovalConflictV1::InvalidEntityIdentity
        | Revision3DialogVoiceSlotRemovalConflictV1::InvalidExpectedLocId => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_REJECTED"
        }
        Revision3DialogVoiceSlotRemovalConflictV1::InvalidDialogLine { .. }
        | Revision3DialogVoiceSlotRemovalConflictV1::DialogLineRevisionConflict { .. } => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_LINE_CONFLICT"
        }
        Revision3DialogVoiceSlotRemovalConflictV1::InvalidLocalizationReference { .. }
        | Revision3DialogVoiceSlotRemovalConflictV1::InvalidLocalization { .. } => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_LOCALIZATION_CONFLICT"
        }
        Revision3DialogVoiceSlotRemovalConflictV1::LocalizationIdentityMismatch { .. } => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_LOC_ID_CONFLICT"
        }
        Revision3DialogVoiceSlotRemovalConflictV1::VoiceSlotIdentityMismatch { .. }
        | Revision3DialogVoiceSlotRemovalConflictV1::InvalidVoiceSlot { .. }
        | Revision3DialogVoiceSlotRemovalConflictV1::VoiceSlotRevisionConflict { .. }
        | Revision3DialogVoiceSlotRemovalConflictV1::VoiceSlotOriginMismatch { .. } => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_SLOT_CONFLICT"
        }
        Revision3DialogVoiceSlotRemovalConflictV1::VoiceSlotHasCandidates { .. }
        | Revision3DialogVoiceSlotRemovalConflictV1::VoiceSlotHasSelection { .. } => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_NOT_EMPTY"
        }
        Revision3DialogVoiceSlotRemovalConflictV1::InvalidLocalBacklink { .. } => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_BACKLINK_CONFLICT"
        }
        Revision3DialogVoiceSlotRemovalConflictV1::ReferenceLimit { .. } => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REFERENCE_LIMIT"
        }
        Revision3DialogVoiceSlotRemovalConflictV1::CandidateTooLarge { .. } => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_PROJECT_LIMIT"
        }
        Revision3DialogVoiceSlotRemovalConflictV1::CandidateNotPersistable { .. } => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_PROJECT_INVALID"
        }
    };
    Failure::new(code, error.to_string())
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::InvalidLimits(_) => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_STORE_LIMITS_INVALID"
        }
        WorkingStoreError::MissingRoot(_) => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_STORE_ROOT_MISSING"
        }
        WorkingStoreError::UnsafePath { .. } => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_STORE_PATH_UNSAFE"
        }
        WorkingStoreError::LimitExceeded { .. } => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_STORE_LIMIT"
        }
        WorkingStoreError::HeadConflict { .. } => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_HEAD_CONFLICT"
        }
        WorkingStoreError::MissingHead(_) => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_HEAD_MISSING"
        }
        WorkingStoreError::MissingObject(_) => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_STORE_OBJECT_MISSING"
        }
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_STORE_COLLISION"
        }
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_STORE_IO"
        }
    };
    Failure::new(
        code,
        format!("dialog VoiceSlot removal Store failure: {error}"),
    )
}

fn truncate_utf8(mut value: String, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value;
    }
    let mut boundary = max_bytes;
    while boundary > 0 && !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
    value
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::path::{Path, PathBuf};

    use gore_authoring::model_revision3::{
        DialogLine, Entity, LocalizationEntry, OriginRef, SchemaRevisionV3, TypedRef, VoiceSlot,
    };
    use gore_authoring::{
        AssetStoreIndex, ContentSeal, FormatV2, GameGenerationAnchor, ProjectId, ProjectMeta,
        Sha256Digest,
    };
    use tempfile::TempDir;

    use super::*;

    const LOCALIZATION_ID_BYTE: u8 = 0x42;
    const LINE_ID_BYTE: u8 = 0x43;
    const SLOT_ID_BYTE: u8 = 0x44;
    const LOC_ID: &str = "GRD_263_ASGHAN_OPEN_INFO_06_02";

    struct PublishedStore {
        temp: TempDir,
        project: ProjectRevision3,
        project_json: String,
        head: WorkingHead,
        fixed_head_bytes: Vec<u8>,
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
            importer: "dialog-voice-slot-removal-ffi-tests".to_owned(),
            source_seal: ContentSeal {
                byte_len: 10,
                sha256: Sha256Digest::from_bytes([tag; 32]),
            },
            external_identity: None,
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

    fn project(revision: u64, generated_slot: bool) -> ProjectRevision3 {
        let pid = project_id(0x51);
        let mut slot = entity(
            SLOT_ID_BYTE,
            3,
            "Asghan German Voice",
            EntityPayload::VoiceSlot(VoiceSlot {
                locale: locale(),
                target_resolution: VoiceTargetResolution::Unresolved,
                candidates: Vec::new(),
                selected: None,
            }),
        );
        if generated_slot {
            slot.origin = OriginRef::Generated {
                generator_id: REVISION3_VOICE_SLOT_GENERATOR_ID_V1.to_owned(),
                generator_version: REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1,
                owner: TypedRef::new(pid, id(LINE_ID_BYTE), EntityKind::DialogLine),
            };
        }
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: pid,
            revision,
            meta: ProjectMeta {
                name: "Dialog VoiceSlot removal FFI fixture".to_owned(),
                version: "1.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: target(),
            authoring_locales: BTreeSet::from([locale()]),
            entities: BTreeMap::from([
                (
                    id(LOCALIZATION_ID_BYTE),
                    entity(
                        LOCALIZATION_ID_BYTE,
                        2,
                        "Asghan line text",
                        EntityPayload::LocalizationEntry(LocalizationEntry {
                            loc_id: LOC_ID.to_owned(),
                            texts: BTreeMap::from([(
                                locale(),
                                "Niemand betritt die Mine.".to_owned(),
                            )]),
                        }),
                    ),
                ),
                (
                    id(LINE_ID_BYTE),
                    entity(
                        LINE_ID_BYTE,
                        5,
                        "Mine entrance warning",
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
                (id(SLOT_ID_BYTE), slot),
            ]),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn publish(project: ProjectRevision3) -> PublishedStore {
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

    fn published_store() -> PublishedStore {
        publish(project(7, true))
    }

    fn request(store: &PublishedStore) -> Revision3DialogVoiceSlotRemovalRequestV1 {
        Revision3DialogVoiceSlotRemovalRequestV1 {
            expected_head: store.head.clone(),
            expected_project_id: store.project.project_id,
            expected_revision: store.project.revision,
            expected_target: store.project.target.clone(),
            line_id: id(LINE_ID_BYTE),
            expected_line_revision: store.project.entities[&id(LINE_ID_BYTE)].revision,
            localization_id: id(LOCALIZATION_ID_BYTE),
            expected_loc_id: LOC_ID.to_owned(),
            locale: locale(),
            slot_id: id(SLOT_ID_BYTE),
            expected_slot_revision: store.project.entities[&id(SLOT_ID_BYTE)].revision,
        }
    }

    fn raw_request(payload: Value) -> String {
        serde_json::to_string(&json!({"command": COMMAND, "payload": payload})).unwrap()
    }

    fn wire(root: &Path, project_json: &str, request_json: &str) -> String {
        serde_json::to_string(&ExactWireRequest {
            command: COMMAND.to_owned(),
            payload: PrepareDialogVoiceSlotRemovalWirePayload {
                current_project_json: project_json.to_owned(),
                root: root.to_string_lossy().into_owned(),
                dialog_voice_slot_removal_request_json: request_json.to_owned(),
            },
        })
        .unwrap()
    }

    fn call(store: &PublishedStore, request: &Revision3DialogVoiceSlotRemovalRequestV1) -> Value {
        crate::dispatch(&wire(
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
                let kind = entry.file_type().unwrap();
                if kind.is_dir() {
                    visit(root, &path, output);
                } else if kind.is_file() {
                    output.insert(
                        path.strip_prefix(root).unwrap().to_owned(),
                        fs::read(path).unwrap(),
                    );
                }
            }
        }
        let mut output = BTreeMap::new();
        visit(root, root, &mut output);
        output
    }

    fn reopen_response(store: &PublishedStore, response: &Value) -> ProjectRevision3 {
        let head_json = response["head_json"].as_str().unwrap();
        let opened = WorkingProjectStore::open_existing(store.temp.path(), ffi_store_limits())
            .unwrap()
            .open_revision3_head_bytes(head_json.as_bytes(), AssetVerification::Full)
            .unwrap();
        assert_eq!(
            opened.project.to_canonical_json().unwrap(),
            response["project_json"]
        );
        opened.project
    }

    fn rival_head(store: &PublishedStore, label: &str) -> Vec<u8> {
        let mut rival = store.project.clone();
        rival.revision += 1;
        rival.meta.name = label.to_owned();
        WorkingProjectStore::open_existing(store.temp.path(), ffi_store_limits())
            .unwrap()
            .prepare_revision3_checkpoint(Some(&store.head), &rival)
            .unwrap()
            .head_bytes
    }

    #[test]
    fn exact_dispatch_prepares_only_the_bound_edge_and_empty_slot_without_publishing() {
        let store = published_store();
        let before_files = snapshot_regular_files(store.temp.path());
        let response = call(&store, &request(&store));
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["outcome"], "prepared_unpublished");
        assert_eq!(
            response["basis_head_json"],
            serde_json::to_string(&store.head).unwrap()
        );
        assert_ne!(response["head_json"], response["basis_head_json"]);
        assert_eq!(response["project_id"], store.project.project_id.to_string());
        assert_eq!(response["revision"], store.project.revision + 1);
        assert_eq!(response["line_id"], id(LINE_ID_BYTE).to_string());
        assert_eq!(response["line_revision"], 6);
        assert_eq!(
            response["localization_id"],
            id(LOCALIZATION_ID_BYTE).to_string()
        );
        assert_eq!(response["slot_id"], id(SLOT_ID_BYTE).to_string());
        assert_eq!(response["removed_slot_revision"], 3);
        assert_eq!(response["locale"], "de");
        assert_eq!(response["loc_id"], LOC_ID);
        assert_eq!(response["removed_target_resolution"], "unresolved");
        assert_eq!(response["build_status"], "blocked");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["target_authority"], "not_granted");
        assert_eq!(response["publication_status"], "not_supported");
        assert_eq!(
            response
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "basis_head_json",
                "build_status",
                "head_json",
                "line_id",
                "line_revision",
                "loc_id",
                "locale",
                "localization_id",
                "ok",
                "outcome",
                "project_id",
                "project_json",
                "publication_status",
                "removed_slot_revision",
                "removed_target_resolution",
                "revision",
                "runtime_status",
                "slot_id",
                "target_authority",
            ])
        );

        let candidate = reopen_response(&store, &response);
        assert_eq!(candidate.revision, store.project.revision + 1);
        assert!(!candidate.entities.contains_key(&id(SLOT_ID_BYTE)));
        let EntityPayload::DialogLine(line) = &candidate.entities[&id(LINE_ID_BYTE)].payload else {
            panic!("expected DialogLine")
        };
        assert!(!line.voice_slots.contains_key(&locale()));
        assert_eq!(candidate.entities[&id(LINE_ID_BYTE)].revision, 6);
        assert_eq!(
            candidate.entities[&id(LOCALIZATION_ID_BYTE)],
            store.project.entities[&id(LOCALIZATION_ID_BYTE)]
        );
        assert_eq!(candidate.asset_store, store.project.asset_store);
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
        assert!(snapshot_regular_files(store.temp.path()).len() > before_files.len());
    }

    #[test]
    fn removal_does_not_require_an_unrelated_localization_text_row() {
        let mut basis = project(7, true);
        let EntityPayload::LocalizationEntry(localization) = &mut basis
            .entities
            .get_mut(&id(LOCALIZATION_ID_BYTE))
            .unwrap()
            .payload
        else {
            panic!("expected LocalizationEntry")
        };
        localization.texts.clear();
        let store = publish(basis);

        let response = call(&store, &request(&store));

        assert_eq!(response["ok"], true, "{response}");
        let candidate = reopen_response(&store, &response);
        let EntityPayload::LocalizationEntry(localization) =
            &candidate.entities[&id(LOCALIZATION_ID_BYTE)].payload
        else {
            panic!("expected LocalizationEntry")
        };
        assert!(localization.texts.is_empty());
    }

    #[test]
    fn malformed_extra_authority_and_oversize_requests_write_nothing() {
        let store = published_store();
        let before = snapshot_regular_files(store.temp.path());
        let request_json = request(&store).to_canonical_json().unwrap();
        let malformed = format!(
            "{} ",
            wire(store.temp.path(), &store.project_json, &request_json)
        );
        let response = prepare_revision3_dialog_voice_slot_removal_v1_raw(&malformed);
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_INVALID"
        );

        let smuggled = raw_request(json!({
            "current_project_json": store.project_json,
            "root": store.temp.path(),
            "dialog_voice_slot_removal_request_json": request_json,
            "game_root": "C:/Games/GothicRemake",
        }));
        let response = prepare_revision3_dialog_voice_slot_removal_v1_raw(&smuggled);
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_INVALID"
        );

        let oversized = wire(
            store.temp.path(),
            &store.project_json,
            &"x".repeat(MAX_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_JSON_BYTES_V1 + 1),
        );
        let response = prepare_revision3_dialog_voice_slot_removal_v1_raw(&oversized);
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_LIMIT"
        );
        assert_eq!(snapshot_regular_files(store.temp.path()), before);
    }

    #[test]
    fn stale_graph_bindings_and_current_project_mismatch_write_no_candidate_objects() {
        for mutate in [
            |request: &mut Revision3DialogVoiceSlotRemovalRequestV1| {
                request.expected_line_revision += 1
            },
            |request: &mut Revision3DialogVoiceSlotRemovalRequestV1| {
                request.expected_slot_revision += 1
            },
            |request: &mut Revision3DialogVoiceSlotRemovalRequestV1| {
                request.expected_loc_id.push('X')
            },
        ] {
            let store = published_store();
            let before = snapshot_regular_files(store.temp.path());
            let mut stale = request(&store);
            mutate(&mut stale);
            let response = call(&store, &stale);
            assert_eq!(response["ok"], false);
            assert_eq!(snapshot_regular_files(store.temp.path()), before);
            assert_eq!(fixed_head(&store), store.fixed_head_bytes);
        }

        let store = published_store();
        let before = snapshot_regular_files(store.temp.path());
        let mut different = store.project.clone();
        different.meta.name = "Unpublished caller bytes".to_owned();
        let response = prepare_revision3_dialog_voice_slot_removal_v1_raw(&wire(
            store.temp.path(),
            &different.to_canonical_json().unwrap(),
            &request(&store).to_canonical_json().unwrap(),
        ));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_PROJECT_CONFLICT"
        );
        assert_eq!(snapshot_regular_files(store.temp.path()), before);
    }

    #[test]
    fn nonmanaged_slot_origin_is_a_closed_conflict_and_writes_nothing() {
        let store = publish(project(7, false));
        let before = snapshot_regular_files(store.temp.path());
        let response = call(&store, &request(&store));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_SLOT_CONFLICT"
        );
        assert_eq!(snapshot_regular_files(store.temp.path()), before);
        assert_eq!(fixed_head(&store), store.fixed_head_bytes);
    }

    #[test]
    fn all_three_head_drift_guards_preserve_the_external_publisher() {
        for gate in 0..3 {
            let store = published_store();
            let rival = rival_head(&store, &format!("External publisher {gate}"));
            let path = store.temp.path().join("gore-project.json");
            let input = wire(
                store.temp.path(),
                &store.project_json,
                &request(&store).to_canonical_json().unwrap(),
            );
            let write = || fs::write(&path, &rival).unwrap();
            let response = match gate {
                0 => prepare_revision3_dialog_voice_slot_removal_v1_inner_with_test_seams(
                    &input,
                    write,
                    || {},
                    || {},
                ),
                1 => prepare_revision3_dialog_voice_slot_removal_v1_inner_with_test_seams(
                    &input,
                    || {},
                    write,
                    || {},
                ),
                _ => prepare_revision3_dialog_voice_slot_removal_v1_inner_with_test_seams(
                    &input,
                    || {},
                    || {},
                    write,
                ),
            }
            .unwrap_err()
            .response();
            assert_eq!(
                response["error"]["code"],
                "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_HEAD_CONFLICT"
            );
            assert_eq!(fs::read(path).unwrap(), rival);
        }
    }

    #[test]
    fn signed_revision_limits_reject_before_candidate_write() {
        let mut value = project(i64::MAX as u64, true);
        value.entities.get_mut(&id(LINE_ID_BYTE)).unwrap().revision = i64::MAX as u64;
        let store = publish(value);
        let before = snapshot_regular_files(store.temp.path());
        let response = call(&store, &request(&store));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REVISION_LIMIT"
        );
        assert_eq!(snapshot_regular_files(store.temp.path()), before);
    }
}
