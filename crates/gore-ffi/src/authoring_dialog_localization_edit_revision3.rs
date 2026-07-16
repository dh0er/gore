//! Exact-current read and prepare-only editing of one managed revision-3 LocalizationEntry.
//!
//! The read route returns the complete bounded locale/text map plus only the DialogLine and
//! VoiceSlot facts needed by a safe editor. The prepare route applies the pure authoring
//! transaction and fully reopens an immutable candidate. Neither route accepts a game or save
//! path, widens the closed preview/content-index wires, publishes the fixed head, or grants
//! topic, build, runtime, deployment, or publication authority.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use gore_authoring::{
    apply_revision3_dialog_localization_edit_transaction_v1, AssetVerification, EntityId,
    LocaleCode, ProjectRevision3, Revision3DialogLocalizationEditBuildStatusV1,
    Revision3DialogLocalizationEditConflictV1, Revision3DialogLocalizationEditErrorV1,
    Revision3DialogLocalizationEditEvaluationV1, Revision3DialogLocalizationEditOutcomeV1,
    Revision3DialogLocalizationEditPublicationStatusV1, Revision3DialogLocalizationEditRequestV1,
    Revision3DialogLocalizationEditRuntimeStatusV1,
    Revision3DialogLocalizationEditTopicAuthorityV1, Revision3EntityKind, Revision3EntityPayload,
    Revision3OriginRef, WorkingHead, WorkingProjectStore, WorkingStoreError, WorkingStoreLimits,
    MAX_PROJECT_JSON_BYTES, MAX_REVISION3_DIALOG_DISPLAY_NAME_BYTES_V1,
    MAX_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_JSON_BYTES_V1,
    MAX_REVISION3_DIALOG_LOCALIZATION_TEXTS_V1, MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_BYTES_V1,
    MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_TOTAL_BYTES_V1,
    MAX_REVISION3_DIALOG_SPEAKER_HINT_BYTES_V1, MAX_REVISION3_VOICE_TARGET_LOC_ID_BYTES_V1,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::err;

pub(super) const READ_COMMAND: &str =
    "authoring_store_read_revision3_dialog_localization_edit_seed_v1";
pub(super) const PREPARE_COMMAND: &str =
    "authoring_store_prepare_revision3_dialog_localization_edit_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_LOC_ID_BYTES: usize = MAX_REVISION3_VOICE_TARGET_LOC_ID_BYTES_V1;
const ENTITY_ID_BYTES: usize = 32;
const MAX_BACKLINKS: usize = 1000;
const MAX_READ_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const MAX_PREPARE_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_BASIS_REVISION: u64 = i64::MAX as u64 - 1;
const MAX_READ_WIRE_BYTES: usize =
    (MAX_PATH_BYTES + MAX_HEAD_JSON_BYTES + MAX_LOC_ID_BYTES + ENTITY_ID_BYTES) * 6 + 4 * 1024;
// Nested canonical JSON strings need at most one additional JSON escape byte per source byte.
// The path retains the full six-byte JSON escape allowance.
const MAX_PREPARE_WIRE_BYTES: usize = MAX_PROJECT_JSON_BYTES * 2
    + MAX_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_JSON_BYTES_V1 * 2
    + MAX_PATH_BYTES * 6
    + 4 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

/// Field order is part of the exact canonical read transport.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReadSeedWirePayload {
    root: String,
    expected_head_json: String,
    localization_id: String,
    expected_localization_revision: u64,
    expected_loc_id: String,
}

/// Field order is part of the exact canonical prepare transport.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PrepareEditWirePayload {
    current_project_json: String,
    localization_edit_request_json: String,
    root: String,
}

#[derive(Debug, Serialize)]
struct LocaleSeed {
    locale: String,
    text: String,
    voice_slot_present: bool,
    candidate_count: u64,
}

#[derive(Debug, Serialize)]
struct LineBacklink {
    line_id: String,
    line_revision: u64,
    display_name: String,
    speaker_hint: Option<String>,
    voice_slot_locales: Vec<String>,
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

pub(super) fn read_revision3_dialog_localization_edit_seed_v1_raw(input: &str) -> Value {
    read_seed_inner(input).unwrap_or_else(Failure::response)
}

fn read_seed_inner(input: &str) -> Result<Value, Failure> {
    read_seed_inner_with_seam_and_limit(input, || {}, MAX_READ_RESPONSE_BYTES)
}

fn read_seed_inner_with_seam_and_limit<F>(
    input: &str,
    between_full_opens: F,
    response_limit: usize,
) -> Result<Value, Failure>
where
    F: FnOnce(),
{
    let payload: ReadSeedWirePayload = parse_exact_wire(
        input,
        READ_COMMAND,
        MAX_READ_WIRE_BYTES,
        "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_INPUT_LIMIT",
    )?;
    validate_read_payload(&payload)?;
    let expected_head = parse_canonical_head(&payload.expected_head_json)?;
    require_signed_serializable(&expected_head)?;
    let localization_id = parse_entity_id(&payload.localization_id)?;
    signed_wire(
        payload.expected_localization_revision,
        "expected LocalizationEntry revision",
    )?;

    let store = WorkingProjectStore::open_existing(Path::new(&payload.root), ffi_store_limits())
        .map_err(map_store_error)?;
    let before = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if before.head != expected_head {
        return Err(head_conflict());
    }
    require_signed_serializable(&before.project)?;
    validate_seed_basis(
        &before.project,
        localization_id,
        payload.expected_localization_revision,
        &payload.expected_loc_id,
    )?;
    let (locales, line_backlinks) = build_seed_rows(&before.project, localization_id)?;
    let localization = before
        .project
        .entities
        .get(&localization_id)
        .expect("validated LocalizationEntry remains present");
    let Revision3EntityPayload::LocalizationEntry(localization_payload) = &localization.payload
    else {
        unreachable!("validated LocalizationEntry kind remains stable")
    };
    let loc_id = localization_payload.loc_id.clone();
    let localization_revision = localization.revision;

    between_full_opens();

    let after = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if after.head != expected_head || after.head != before.head || after.project != before.project {
        return Err(head_conflict());
    }

    enforce_response_budget(
        json!({
            "ok": true,
            "outcome": "read_only",
            "head_json": payload.expected_head_json,
            "project_id": after.project.project_id.to_string(),
            "project_revision": after.project.revision,
            "localization_id": localization_id.to_string(),
            "localization_revision": localization_revision,
            "loc_id": loc_id,
            "locales": locales,
            "line_backlinks": line_backlinks,
            "content_authority": "read_only_exact_current_localization_edit_seed",
            "build_status": "not_evaluated",
            "runtime_status": "runtime_unqualified",
            "publication_status": "not_applicable",
        }),
        response_limit,
    )
}

pub(super) fn prepare_revision3_dialog_localization_edit_v1_raw(input: &str) -> Value {
    prepare_edit_inner(input).unwrap_or_else(Failure::response)
}

fn prepare_edit_inner(input: &str) -> Result<Value, Failure> {
    prepare_edit_inner_with_test_seams(input, || {}, || {}, MAX_PREPARE_RESPONSE_BYTES)
}

#[cfg(test)]
fn prepare_edit_inner_with_post_prepare_guard<A>(
    input: &str,
    after_checkpoint: A,
) -> Result<Value, Failure>
where
    A: FnOnce(),
{
    prepare_edit_inner_with_test_seams(input, after_checkpoint, || {}, MAX_PREPARE_RESPONSE_BYTES)
}

#[cfg(test)]
fn prepare_edit_inner_with_final_guard<F>(input: &str, final_guard: F) -> Result<Value, Failure>
where
    F: FnOnce(),
{
    prepare_edit_inner_with_test_seams(input, || {}, final_guard, MAX_PREPARE_RESPONSE_BYTES)
}

fn prepare_edit_inner_with_test_seams<A, F>(
    input: &str,
    after_checkpoint: A,
    final_guard: F,
    response_limit: usize,
) -> Result<Value, Failure>
where
    A: FnOnce(),
    F: FnOnce(),
{
    let payload: PrepareEditWirePayload = parse_exact_wire(
        input,
        PREPARE_COMMAND,
        MAX_PREPARE_WIRE_BYTES,
        "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_INPUT_LIMIT",
    )?;
    validate_prepare_payload(&payload)?;
    let request = Revision3DialogLocalizationEditRequestV1::from_json(
        &payload.localization_edit_request_json,
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

    let canonical_basis = basis
        .project
        .to_canonical_json()
        .map_err(|_| invariant_failure())?;
    if canonical_basis.as_bytes() != payload.current_project_json.as_bytes() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_PROJECT_CONFLICT",
            "current_project_json differs from the exact published revision-3 project",
        ));
    }
    bind_request_to_basis(&basis.head, &basis.project, &request)?;

    let outcome = match apply_revision3_dialog_localization_edit_transaction_v1(
        &basis.head,
        &canonical_basis,
        &payload.localization_edit_request_json,
    )
    .map_err(map_transaction_error)?
    {
        Revision3DialogLocalizationEditEvaluationV1::Applied(outcome) => *outcome,
        Revision3DialogLocalizationEditEvaluationV1::Rejected(rejection) => {
            return Err(map_transaction_conflict(rejection.conflict));
        }
    };
    require_signed_serializable(&outcome.project)?;
    verify_outcome_binding(&basis.head, &basis.project, &request, &outcome)?;
    match outcome.build_status {
        Revision3DialogLocalizationEditBuildStatusV1::Blocked => {}
    }
    match outcome.runtime_status {
        Revision3DialogLocalizationEditRuntimeStatusV1::RuntimeUnqualified => {}
    }
    match outcome.topic_authority {
        Revision3DialogLocalizationEditTopicAuthorityV1::NotGranted => {}
    }
    match outcome.publication_status {
        Revision3DialogLocalizationEditPublicationStatusV1::NotSupported => {}
    }

    let prepared = store
        .prepare_revision3_checkpoint(Some(&basis.head), &outcome.project)
        .map_err(map_store_error)?;
    let reopened = store
        .open_revision3_head_bytes(&prepared.head_bytes, AssetVerification::Full)
        .map_err(map_store_error)?;
    if reopened.head != prepared.head || reopened.project != outcome.project {
        return Err(invariant_failure());
    }
    let reopened_json = reopened
        .project
        .to_canonical_json()
        .map_err(|_| invariant_failure())?;
    if reopened_json != outcome.canonical_project_json {
        return Err(invariant_failure());
    }

    after_checkpoint();
    require_fixed_basis(&store, &basis.head, &basis.project)?;

    let basis_head_json = canonical_head_json(&basis.head)?;
    let candidate_head_json = String::from_utf8(prepared.head_bytes).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_INVARIANT",
            "the prepared localization-edit head is not UTF-8 JSON",
        )
    })?;
    if candidate_head_json.is_empty() || candidate_head_json.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_RESPONSE_LIMIT",
            "the prepared localization-edit head exceeds its bounded transport limit",
        ));
    }
    if serde_json::to_string(&prepared.head).ok().as_deref() != Some(&candidate_head_json) {
        return Err(invariant_failure());
    }
    require_signed_serializable(&prepared.head)?;
    let localization_revision = outcome
        .project
        .entities
        .get(&outcome.localization_id)
        .ok_or_else(invariant_failure)?
        .revision;
    signed_wire(
        localization_revision,
        "candidate LocalizationEntry revision",
    )?;

    let response = json!({
        "ok": true,
        "outcome": "prepared_unpublished",
        "basis_head_json": basis_head_json,
        "head_json": candidate_head_json,
        "project_json": outcome.canonical_project_json,
        "project_id": outcome.project.project_id.to_string(),
        "revision": outcome.project.revision,
        "localization_id": outcome.localization_id.to_string(),
        "localization_revision": localization_revision,
        "added_locales": outcome.added_locales,
        "removed_locales": outcome.removed_locales,
        "build_status": "blocked",
        "runtime_status": "runtime_unqualified",
        "topic_authority": "not_granted",
        "publication_status": "not_supported",
    });
    let response = enforce_response_budget(response, response_limit)?;

    final_guard();
    require_fixed_basis(&store, &basis.head, &basis.project)?;
    Ok(response)
}

fn parse_exact_wire<P>(
    input: &str,
    command: &'static str,
    limit: usize,
    limit_code: &'static str,
) -> Result<P, Failure>
where
    P: DeserializeOwned + Serialize,
{
    if input.len() > limit {
        return Err(Failure::new(
            limit_code,
            format!("revision-3 localization-edit request exceeds the {limit}-byte wire limit"),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != command {
        return Err(invalid_request());
    }
    let canonical = serde_json::to_string(&request).map_err(|_| invariant_failure())?;
    if canonical.as_bytes() != input.as_bytes() {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_read_payload(payload: &ReadSeedWirePayload) -> Result<(), Failure> {
    validate_path(&payload.root)?;
    validate_loc_id(&payload.expected_loc_id)?;
    if payload.expected_head_json.is_empty()
        || payload.expected_head_json.len() > MAX_HEAD_JSON_BYTES
    {
        return Err(head_invalid());
    }
    Ok(())
}

fn validate_prepare_payload(payload: &PrepareEditWirePayload) -> Result<(), Failure> {
    validate_path(&payload.root)?;
    if payload.current_project_json.is_empty() || payload.localization_edit_request_json.is_empty()
    {
        return Err(invalid_request());
    }
    if payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_PROJECT_LIMIT",
            format!("current_project_json exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ));
    }
    if payload.localization_edit_request_json.len()
        > MAX_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_JSON_BYTES_V1
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_LIMIT",
            format!(
                "localization_edit_request_json exceeds the {MAX_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_JSON_BYTES_V1}-byte limit"
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

fn validate_loc_id(loc_id: &str) -> Result<(), Failure> {
    if loc_id.is_empty() || loc_id.len() > MAX_LOC_ID_BYTES || loc_id.contains('\0') {
        return Err(invalid_request());
    }
    Ok(())
}

fn parse_entity_id(input: &str) -> Result<EntityId, Failure> {
    if input.len() != ENTITY_ID_BYTES {
        return Err(invalid_request());
    }
    let id = input.parse::<EntityId>().map_err(|_| invalid_request())?;
    if id.to_string() != input {
        return Err(invalid_request());
    }
    Ok(id)
}

fn parse_canonical_head(input: &str) -> Result<WorkingHead, Failure> {
    let head: WorkingHead = serde_json::from_str(input).map_err(|_| head_invalid())?;
    if serde_json::to_string(&head).ok().as_deref() != Some(input) {
        return Err(head_invalid());
    }
    Ok(head)
}

fn validate_seed_basis(
    project: &ProjectRevision3,
    localization_id: EntityId,
    expected_revision: u64,
    expected_loc_id: &str,
) -> Result<(), Failure> {
    signed_wire(project.revision, "project revision")?;
    let entity = project
        .entities
        .get(&localization_id)
        .ok_or_else(localization_not_found)?;
    let Revision3EntityPayload::LocalizationEntry(localization) = &entity.payload else {
        return Err(localization_not_found());
    };
    signed_wire(entity.revision, "LocalizationEntry revision")?;
    if entity.revision != expected_revision {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REVISION_CONFLICT",
            "the exact managed LocalizationEntry revision changed",
        ));
    }
    if localization.loc_id != expected_loc_id {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_IDENTITY_CONFLICT",
            "the exact managed LocalizationEntry identity changed",
        ));
    }
    if !matches!(entity.origin, Revision3OriginRef::New { .. }) {
        return Err(origin_conflict());
    }
    validate_seed_texts(&localization.texts)
}

fn validate_seed_texts(texts: &BTreeMap<LocaleCode, String>) -> Result<(), Failure> {
    if texts.is_empty() || texts.len() > MAX_REVISION3_DIALOG_LOCALIZATION_TEXTS_V1 {
        return Err(text_limit());
    }
    let mut total = 0usize;
    let mut has_nonblank = false;
    for text in texts.values() {
        total = total.checked_add(text.len()).ok_or_else(text_limit)?;
        has_nonblank |= !text.trim().is_empty();
        if text.len() > MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_BYTES_V1
            || text.contains('\0')
            || total > MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_TOTAL_BYTES_V1
        {
            return Err(text_limit());
        }
    }
    if !has_nonblank {
        return Err(text_limit());
    }
    Ok(())
}

fn build_seed_rows(
    project: &ProjectRevision3,
    localization_id: EntityId,
) -> Result<(Vec<LocaleSeed>, Vec<LineBacklink>), Failure> {
    let entity = project
        .entities
        .get(&localization_id)
        .ok_or_else(localization_not_found)?;
    let Revision3EntityPayload::LocalizationEntry(localization) = &entity.payload else {
        return Err(localization_not_found());
    };
    let mut voice_locales = BTreeSet::new();
    let mut candidate_counts = localization
        .texts
        .keys()
        .cloned()
        .map(|locale| (locale, 0u64))
        .collect::<BTreeMap<_, _>>();
    let mut backlinks = Vec::new();

    for (line_id, line_entity) in &project.entities {
        let Revision3EntityPayload::DialogLine(line) = &line_entity.payload else {
            continue;
        };
        if line.localization.project_id != project.project_id
            || line.localization.expected_kind != Revision3EntityKind::LocalizationEntry
            || line.localization.id != localization_id
        {
            continue;
        }
        if backlinks.len() >= MAX_BACKLINKS {
            return Err(Failure::new(
                "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_BACKLINK_LIMIT",
                format!("the exact LocalizationEntry has more than {MAX_BACKLINKS} DialogLine backlinks"),
            ));
        }
        signed_wire(line_entity.revision, "DialogLine revision")?;
        if !valid_display_name(&line_entity.display_name) {
            return Err(invariant_failure());
        }
        if line
            .speaker_hint
            .as_deref()
            .is_some_and(|speaker| !valid_speaker_hint(speaker))
        {
            return Err(invariant_failure());
        }

        let mut slot_locales = Vec::with_capacity(line.voice_slots.len());
        for (locale, slot_reference) in &line.voice_slots {
            let Some(total) = candidate_counts.get_mut(locale) else {
                return Err(invariant_failure());
            };
            let slot_entity = project
                .entities
                .get(&slot_reference.id)
                .ok_or_else(invariant_failure)?;
            let Revision3EntityPayload::VoiceSlot(slot) = &slot_entity.payload else {
                return Err(invariant_failure());
            };
            if slot_reference.project_id != project.project_id
                || slot_reference.expected_kind != Revision3EntityKind::VoiceSlot
                || slot.locale != *locale
            {
                return Err(invariant_failure());
            }
            let count = u64::try_from(slot.candidates.len()).map_err(|_| invariant_failure())?;
            *total = total.checked_add(count).ok_or_else(invariant_failure)?;
            signed_wire(*total, "VoiceTake candidate count")?;
            voice_locales.insert(locale.clone());
            slot_locales.push(locale.to_string());
        }
        backlinks.push(LineBacklink {
            line_id: line_id.to_string(),
            line_revision: line_entity.revision,
            display_name: line_entity.display_name.clone(),
            speaker_hint: line.speaker_hint.clone(),
            voice_slot_locales: slot_locales,
        });
    }

    let locales = localization
        .texts
        .iter()
        .map(|(locale, text)| LocaleSeed {
            locale: locale.to_string(),
            text: text.clone(),
            voice_slot_present: voice_locales.contains(locale),
            candidate_count: candidate_counts[locale],
        })
        .collect();
    Ok((locales, backlinks))
}

fn bind_request_to_basis(
    head: &WorkingHead,
    project: &ProjectRevision3,
    request: &Revision3DialogLocalizationEditRequestV1,
) -> Result<(), Failure> {
    if request.expected_head != *head {
        return Err(head_conflict());
    }
    if request.expected_project_id != project.project_id
        || request.expected_revision != project.revision
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_PROJECT_CONFLICT",
            "the localization-edit request project differs from the exact published project",
        ));
    }
    if request.expected_target != project.target {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_TARGET_CONFLICT",
            "the localization-edit request target differs from the exact published project target",
        ));
    }
    let entity = project
        .entities
        .get(&request.localization_id)
        .ok_or_else(localization_not_found)?;
    let Revision3EntityPayload::LocalizationEntry(localization) = &entity.payload else {
        return Err(localization_not_found());
    };
    if entity.revision != request.expected_localization_revision {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REVISION_CONFLICT",
            "the localization-edit request entity revision is stale",
        ));
    }
    if localization.loc_id != request.expected_loc_id {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_IDENTITY_CONFLICT",
            "the localization-edit request entity identity is stale",
        ));
    }
    if !matches!(entity.origin, Revision3OriginRef::New { .. }) {
        return Err(origin_conflict());
    }
    Ok(())
}

fn verify_outcome_binding(
    basis_head: &WorkingHead,
    basis: &ProjectRevision3,
    request: &Revision3DialogLocalizationEditRequestV1,
    outcome: &Revision3DialogLocalizationEditOutcomeV1,
) -> Result<(), Failure> {
    let expected_project_revision = basis
        .revision
        .checked_add(1)
        .ok_or_else(|| revision_limit("the project revision cannot be incremented"))?;
    let expected_localization_revision = request
        .expected_localization_revision
        .checked_add(1)
        .ok_or_else(|| revision_limit("the LocalizationEntry revision cannot be incremented"))?;
    let basis_entity = basis
        .entities
        .get(&request.localization_id)
        .ok_or_else(invariant_failure)?;
    let Revision3EntityPayload::LocalizationEntry(basis_localization) = &basis_entity.payload
    else {
        return Err(invariant_failure());
    };
    let basis_locales = basis_localization
        .texts
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let replacement_locales = request.texts.keys().cloned().collect::<BTreeSet<_>>();
    let expected_added = replacement_locales
        .difference(&basis_locales)
        .cloned()
        .collect::<BTreeSet<_>>();
    let expected_removed = basis_locales
        .difference(&replacement_locales)
        .cloned()
        .collect::<BTreeSet<_>>();

    let mut expected_project = basis.clone();
    expected_project.revision = expected_project_revision;
    expected_project
        .authoring_locales
        .extend(expected_added.iter().cloned());
    let expected_entity = expected_project
        .entities
        .get_mut(&request.localization_id)
        .ok_or_else(invariant_failure)?;
    expected_entity.revision = expected_localization_revision;
    let Revision3EntityPayload::LocalizationEntry(expected_localization) =
        &mut expected_entity.payload
    else {
        return Err(invariant_failure());
    };
    expected_localization.texts.clone_from(&request.texts);

    if outcome.basis_head != *basis_head
        || outcome.localization_id != request.localization_id
        || outcome.added_locales != expected_added
        || outcome.removed_locales != expected_removed
        || outcome.project != expected_project
    {
        return Err(invariant_failure());
    }
    let canonical = outcome
        .project
        .to_canonical_json()
        .map_err(|_| invariant_failure())?;
    if canonical != outcome.canonical_project_json {
        return Err(invariant_failure());
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
    let value = serde_json::to_value(value).map_err(|_| invariant_failure())?;
    require_signed_json_value(&value)
}

fn require_signed_json_value(value: &Value) -> Result<(), Failure> {
    match value {
        Value::Number(number) if number.as_u64().is_some_and(|value| value > i64::MAX as u64) => {
            Err(Failure::new(
                "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_SIGNED_WIRE_LIMIT",
                "a localization-edit wire integer exceeds the signed 64-bit transport range",
            ))
        }
        Value::Array(values) => values.iter().try_for_each(require_signed_json_value),
        Value::Object(values) => values.values().try_for_each(require_signed_json_value),
        _ => Ok(()),
    }
}

fn signed_wire(value: u64, field: &'static str) -> Result<(), Failure> {
    if value > i64::MAX as u64 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_SIGNED_WIRE_LIMIT",
            format!("{field} exceeds the signed 64-bit transport range"),
        ));
    }
    Ok(())
}

fn canonical_head_json(head: &WorkingHead) -> Result<String, Failure> {
    require_signed_serializable(head)?;
    let value = serde_json::to_string(head).map_err(|_| invariant_failure())?;
    if value.is_empty() || value.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_RESPONSE_LIMIT",
            "the localization-edit basis head exceeds its bounded transport limit",
        ));
    }
    Ok(value)
}

fn enforce_response_budget(response: Value, limit: usize) -> Result<Value, Failure> {
    let encoded = serde_json::to_vec(&response).map_err(|_| invariant_failure())?;
    if encoded.len() > limit {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_RESPONSE_LIMIT",
            "the localization-edit response exceeds its bounded transport budget",
        ));
    }
    Ok(response)
}

fn valid_display_name(value: &str) -> bool {
    value.trim() == value
        && !value.is_empty()
        && value.len() <= MAX_REVISION3_DIALOG_DISPLAY_NAME_BYTES_V1
        && !value.chars().any(char::is_control)
}

fn valid_speaker_hint(value: &str) -> bool {
    value.trim() == value
        && !value.is_empty()
        && value.len() <= MAX_REVISION3_DIALOG_SPEAKER_HINT_BYTES_V1
        && !value.chars().any(char::is_control)
}

fn ffi_store_limits() -> WorkingStoreLimits {
    WorkingStoreLimits {
        max_referenced_entity_bytes: MAX_PROJECT_JSON_BYTES as u64,
        ..WorkingStoreLimits::default()
    }
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_INVALID",
        "request must contain one exact duplicate-free canonical command and only the closed localization-edit payload fields",
    )
}

fn head_invalid() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_HEAD_INVALID",
        "expected_head_json is not one bounded duplicate-free canonical revision-3 working head",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_HEAD_CONFLICT",
        "the published revision-3 head or exact project changed during localization editing",
    )
}

fn localization_not_found() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_NOT_FOUND",
        "the exact managed LocalizationEntry is missing or has the wrong entity kind",
    )
}

fn origin_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_ORIGIN_CONFLICT",
        "only a newly authored managed LocalizationEntry can be edited",
    )
}

fn text_limit() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_TEXT_LIMIT",
        "the exact LocalizationEntry texts are empty, invalid, or exceed their closed budget",
    )
}

fn revision_limit(message: &'static str) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REVISION_LIMIT",
        message,
    )
}

fn invariant_failure() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_INVARIANT",
        "the exact localization-edit operation violated its closed native invariants",
    )
}

fn map_request_error(error: impl std::fmt::Display) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_INVALID",
        format!("the exact localization-edit request is invalid: {error}"),
    )
}

fn map_transaction_error(error: Revision3DialogLocalizationEditErrorV1) -> Failure {
    match error {
        Revision3DialogLocalizationEditErrorV1::InvalidProject(_) => Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_PROJECT_INVALID",
            "the exact current project is not a valid localization-edit basis",
        ),
        Revision3DialogLocalizationEditErrorV1::InvalidRequest(error) => map_request_error(error),
        Revision3DialogLocalizationEditErrorV1::ReopenCandidate(_)
        | Revision3DialogLocalizationEditErrorV1::CanonicalReopenMismatch => invariant_failure(),
    }
}

fn map_transaction_conflict(error: Revision3DialogLocalizationEditConflictV1) -> Failure {
    let code = match &error {
        Revision3DialogLocalizationEditConflictV1::CurrentHeadMismatch => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_HEAD_CONFLICT"
        }
        Revision3DialogLocalizationEditConflictV1::ProjectIdentityMismatch { .. }
        | Revision3DialogLocalizationEditConflictV1::ProjectRevisionConflict { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_PROJECT_CONFLICT"
        }
        Revision3DialogLocalizationEditConflictV1::ProjectTargetMismatch => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_TARGET_CONFLICT"
        }
        Revision3DialogLocalizationEditConflictV1::LocalizationMissingOrWrongKind { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_NOT_FOUND"
        }
        Revision3DialogLocalizationEditConflictV1::LocalizationRevisionConflict { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REVISION_CONFLICT"
        }
        Revision3DialogLocalizationEditConflictV1::LocalizationIdentityConflict { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_IDENTITY_CONFLICT"
        }
        Revision3DialogLocalizationEditConflictV1::LocalizationOriginNotNew { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_ORIGIN_CONFLICT"
        }
        Revision3DialogLocalizationEditConflictV1::InvalidLocalizationTexts => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_REJECTED"
        }
        Revision3DialogLocalizationEditConflictV1::NoChanges => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_NO_CHANGES"
        }
        Revision3DialogLocalizationEditConflictV1::ProjectRevisionOverflow
        | Revision3DialogLocalizationEditConflictV1::LocalizationRevisionOverflow { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REVISION_LIMIT"
        }
        Revision3DialogLocalizationEditConflictV1::VoiceSlotLocaleRemovedOrBlank { .. }
        | Revision3DialogLocalizationEditConflictV1::VoiceSlotCandidatesProtectText { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_VOICE_CONFLICT"
        }
        Revision3DialogLocalizationEditConflictV1::CandidateTooLarge { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_PROJECT_LIMIT"
        }
        Revision3DialogLocalizationEditConflictV1::CandidateNotPersistable { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_PROJECT_INVALID"
        }
    };
    Failure::new(code, error.to_string())
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::InvalidLimits(_) => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_STORE_LIMITS_INVALID"
        }
        WorkingStoreError::MissingRoot(_) => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_STORE_ROOT_MISSING"
        }
        WorkingStoreError::UnsafePath { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_STORE_PATH_UNSAFE"
        }
        WorkingStoreError::LimitExceeded { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_STORE_LIMIT"
        }
        WorkingStoreError::HeadConflict { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_HEAD_CONFLICT"
        }
        WorkingStoreError::MissingHead(_) => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_HEAD_MISSING"
        }
        WorkingStoreError::MissingObject(_) => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_STORE_OBJECT_MISSING"
        }
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_STORE_COLLISION"
        }
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_STORE_IO"
        }
    };
    Failure::new(
        code,
        "the exact revision-3 localization Store operation failed",
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

    use gore_authoring::model_revision3::{
        DialogLine, LocalizationEntry, OggCodec, OggMetadata, OriginRef, TypedRef, VoiceSlot,
        VoiceTake, VoiceTakeStatus, VoiceTargetResolution,
    };
    use gore_authoring::{
        AssetMeta, AssetStoreIndex, ContentSeal, FormatV2, GameGenerationAnchor, ProjectId,
        ProjectMeta, Revision3Entity, SchemaRevisionV3, Sha256Digest,
    };
    use serde_json::{json, Map};
    use tempfile::TempDir;

    use super::*;

    const LOC_ID: &str = "GRD_263_ASGHAN_OPEN_INFO_06_02";
    const LOCALIZATION_TAG: u8 = 0x41;
    const LINE_TAG: u8 = 0x42;
    const SLOT_TAG: u8 = 0x43;
    const TAKE_TAG: u8 = 0x44;

    struct PublishedStore {
        temp: TempDir,
        project: ProjectRevision3,
        project_json: String,
        head: WorkingHead,
        fixed_head_bytes: Vec<u8>,
        localization_id: EntityId,
    }

    fn id(tag: u8) -> EntityId {
        EntityId::from_bytes([tag; 16])
    }

    fn project_id() -> ProjectId {
        ProjectId::from_bytes([0x31; 16])
    }

    fn target() -> GameGenerationAnchor {
        GameGenerationAnchor {
            executable: ContentSeal {
                byte_len: 171_698_176,
                sha256: Sha256Digest::from_bytes([0x51; 32]),
            },
        }
    }

    fn locale(value: &str) -> LocaleCode {
        value.parse().unwrap()
    }

    fn new_origin(runtime_id: &str) -> OriginRef {
        OriginRef::New {
            authored_runtime_id: runtime_id.to_owned(),
        }
    }

    fn imported_origin(tag: u8) -> OriginRef {
        OriginRef::Imported {
            importer: "localization-edit-ffi-tests".to_owned(),
            source_seal: ContentSeal {
                byte_len: 10,
                sha256: Sha256Digest::from_bytes([tag; 32]),
            },
            external_identity: None,
        }
    }

    fn entity(
        tag: u8,
        revision: u64,
        display_name: &str,
        origin: OriginRef,
        payload: Revision3EntityPayload,
    ) -> Revision3Entity {
        Revision3Entity {
            id: id(tag),
            display_name: display_name.to_owned(),
            origin,
            revision,
            payload,
        }
    }

    fn empty_project(revision: u64) -> ProjectRevision3 {
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: project_id(),
            revision,
            meta: ProjectMeta {
                name: "Localization edit FFI fixture".to_owned(),
                version: "1.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: target(),
            authoring_locales: BTreeSet::from([locale("de"), locale("en")]),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn base_project(
        texts: BTreeMap<LocaleCode, String>,
        localization_origin: OriginRef,
    ) -> ProjectRevision3 {
        let localization_id = id(LOCALIZATION_TAG);
        let mut project = empty_project(7);
        project.entities.insert(
            localization_id,
            entity(
                LOCALIZATION_TAG,
                4,
                "Asghan warning text",
                localization_origin,
                Revision3EntityPayload::LocalizationEntry(LocalizationEntry {
                    loc_id: LOC_ID.to_owned(),
                    texts,
                }),
            ),
        );
        project
    }

    fn add_line(project: &mut ProjectRevision3, tag: u8, voice_slot: Option<(u8, &str)>) {
        let mut voice_slots = BTreeMap::new();
        if let Some((slot_tag, locale_code)) = voice_slot {
            voice_slots.insert(
                locale(locale_code),
                TypedRef::new(project_id(), id(slot_tag), Revision3EntityKind::VoiceSlot),
            );
        }
        project.entities.insert(
            id(tag),
            entity(
                tag,
                2,
                &format!("Asghan warning line {tag:02x}"),
                imported_origin(tag),
                Revision3EntityPayload::DialogLine(DialogLine {
                    localization: TypedRef::new(
                        project_id(),
                        id(LOCALIZATION_TAG),
                        Revision3EntityKind::LocalizationEntry,
                    ),
                    speaker_hint: Some("Asghan".to_owned()),
                    voice_slots,
                }),
            ),
        );
    }

    fn add_voice_graph(project: &mut ProjectRevision3, imported: &gore_authoring::ImportedOgg) {
        add_line(project, LINE_TAG, Some((SLOT_TAG, "de")));
        project.asset_store.assets.insert(
            imported.asset.sha256,
            AssetMeta {
                byte_len: imported.asset.byte_len,
                media_type: "audio/ogg".to_owned(),
            },
        );
        let take_ref = TypedRef::new(project_id(), id(TAKE_TAG), Revision3EntityKind::VoiceTake);
        project.entities.insert(
            id(SLOT_TAG),
            entity(
                SLOT_TAG,
                3,
                "Asghan German Voice slot",
                imported_origin(SLOT_TAG),
                Revision3EntityPayload::VoiceSlot(VoiceSlot {
                    locale: locale("de"),
                    target_resolution: VoiceTargetResolution::Unresolved,
                    candidates: vec![take_ref.clone()],
                    selected: Some(take_ref),
                }),
            ),
        );
        let mut asset = imported.asset.clone();
        asset.logical_name = "GRD_263_ASGHAN_OPEN_INFO_06_02_take_01.ogg".to_owned();
        project.entities.insert(
            id(TAKE_TAG),
            entity(
                TAKE_TAG,
                1,
                "Asghan German take",
                imported_origin(TAKE_TAG),
                Revision3EntityPayload::VoiceTake(VoiceTake {
                    locale: locale("de"),
                    asset,
                    ogg: OggMetadata {
                        codec: match imported.ogg.codec {
                            gore_authoring::OggCodec::Vorbis => OggCodec::Vorbis,
                            gore_authoring::OggCodec::Opus => OggCodec::Opus,
                        },
                        channels: imported.ogg.channels,
                        sample_rate: imported.ogg.sample_rate,
                        pages: imported.ogg.pages,
                        logical_streams: imported.ogg.logical_streams,
                    },
                    status: VoiceTakeStatus::Approved,
                }),
            ),
        );
    }

    fn publish_project(
        mut project: ProjectRevision3,
        with_voice_candidate: bool,
    ) -> PublishedStore {
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let initial = empty_project(project.revision - 1);
        let initial_checkpoint = store.prepare_revision3_checkpoint(None, &initial).unwrap();
        fs::write(
            temp.path().join("gore-project.json"),
            &initial_checkpoint.head_bytes,
        )
        .unwrap();

        if with_voice_candidate {
            let source_temp = TempDir::new().unwrap();
            let source = source_temp.path().join("asghan-localization-edit.ogg");
            fs::write(
                &source,
                include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
            )
            .unwrap();
            let imported = store
                .import_ogg(
                    &source,
                    "asghan-localization-edit.ogg",
                    Some(&initial_checkpoint.head),
                )
                .unwrap();
            add_voice_graph(&mut project, &imported);
        }

        let project_json = project.to_canonical_json().unwrap();
        let published = store
            .prepare_revision3_checkpoint(Some(&initial_checkpoint.head), &project)
            .unwrap();
        fs::write(temp.path().join("gore-project.json"), &published.head_bytes).unwrap();
        PublishedStore {
            temp,
            project,
            project_json,
            head: published.head,
            fixed_head_bytes: published.head_bytes,
            localization_id: id(LOCALIZATION_TAG),
        }
    }

    fn published_store(with_voice_candidate: bool) -> PublishedStore {
        publish_project(
            base_project(
                BTreeMap::from([
                    (locale("de"), "Bleib stehen!".to_owned()),
                    (locale("en"), "Stop right there!".to_owned()),
                ]),
                new_origin(LOC_ID),
            ),
            with_voice_candidate,
        )
    }

    fn read_wire_with(
        store: &PublishedStore,
        head: &WorkingHead,
        localization_id: EntityId,
        revision: u64,
        loc_id: &str,
    ) -> String {
        serde_json::to_string(&ExactWireRequest {
            command: READ_COMMAND.to_owned(),
            payload: ReadSeedWirePayload {
                root: store.temp.path().to_string_lossy().into_owned(),
                expected_head_json: serde_json::to_string(head).unwrap(),
                localization_id: localization_id.to_string(),
                expected_localization_revision: revision,
                expected_loc_id: loc_id.to_owned(),
            },
        })
        .unwrap()
    }

    fn read_wire(store: &PublishedStore) -> String {
        read_wire_with(store, &store.head, store.localization_id, 4, LOC_ID)
    }

    fn edit_request(
        store: &PublishedStore,
        texts: BTreeMap<LocaleCode, String>,
    ) -> Revision3DialogLocalizationEditRequestV1 {
        Revision3DialogLocalizationEditRequestV1 {
            expected_head: store.head.clone(),
            expected_project_id: store.project.project_id,
            expected_revision: store.project.revision,
            expected_target: store.project.target.clone(),
            localization_id: store.localization_id,
            expected_localization_revision: 4,
            expected_loc_id: LOC_ID.to_owned(),
            texts,
        }
    }

    fn prepare_wire_with_json(store: &PublishedStore, request_json: &str) -> String {
        serde_json::to_string(&ExactWireRequest {
            command: PREPARE_COMMAND.to_owned(),
            payload: PrepareEditWirePayload {
                current_project_json: store.project_json.clone(),
                localization_edit_request_json: request_json.to_owned(),
                root: store.temp.path().to_string_lossy().into_owned(),
            },
        })
        .unwrap()
    }

    fn prepare_wire(store: &PublishedStore, texts: BTreeMap<LocaleCode, String>) -> String {
        prepare_wire_with_json(
            store,
            &edit_request(store, texts).to_canonical_json().unwrap(),
        )
    }

    fn error_code(value: &Value) -> &str {
        value["error"]["code"].as_str().unwrap()
    }

    fn file_tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn visit(root: &Path, directory: &Path, output: &mut BTreeMap<PathBuf, Vec<u8>>) {
            let mut entries = fs::read_dir(directory)
                .unwrap()
                .map(|entry| entry.unwrap())
                .collect::<Vec<_>>();
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                let path = entry.path();
                if path.is_dir() {
                    visit(root, &path, output);
                } else if path.is_file() {
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

    fn keys(object: &Map<String, Value>) -> BTreeSet<&str> {
        object.keys().map(String::as_str).collect()
    }

    fn publish_concurrent_change(
        root: &Path,
        basis: &WorkingHead,
        basis_project: &ProjectRevision3,
    ) {
        let mut changed = basis_project.clone();
        changed.revision += 1;
        changed.meta.name = "Concurrent localization project".to_owned();
        let writer = WorkingProjectStore::open_existing(root, ffi_store_limits()).unwrap();
        let prepared = writer
            .prepare_revision3_checkpoint(Some(basis), &changed)
            .unwrap();
        fs::write(root.join("gore-project.json"), prepared.head_bytes).unwrap();
    }

    #[test]
    fn registered_seed_read_returns_full_multibyte_text_and_exact_voice_backlinks_without_writes() {
        let long = format!(
            "{}😀",
            "x".repeat(MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_BYTES_V1 - 4)
        );
        assert_eq!(long.len(), MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_BYTES_V1);
        let store = publish_project(
            base_project(
                BTreeMap::from([
                    (locale("de"), long.clone()),
                    (locale("en"), "Stop right there!".to_owned()),
                ]),
                new_origin(LOC_ID),
            ),
            true,
        );
        let before = file_tree(store.temp.path());

        let response = crate::dispatch(&read_wire(&store));

        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["outcome"], "read_only");
        assert_eq!(
            response["head_json"],
            serde_json::to_string(&store.head).unwrap()
        );
        assert_eq!(response["project_id"], project_id().to_string());
        assert_eq!(response["project_revision"], 7);
        assert_eq!(
            response["localization_id"],
            id(LOCALIZATION_TAG).to_string()
        );
        assert_eq!(response["localization_revision"], 4);
        assert_eq!(response["loc_id"], LOC_ID);
        assert_eq!(
            response["content_authority"],
            "read_only_exact_current_localization_edit_seed"
        );
        assert_eq!(response["build_status"], "not_evaluated");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["publication_status"], "not_applicable");
        assert_eq!(
            keys(response.as_object().unwrap()),
            BTreeSet::from([
                "build_status",
                "content_authority",
                "head_json",
                "line_backlinks",
                "locales",
                "localization_id",
                "localization_revision",
                "loc_id",
                "ok",
                "outcome",
                "project_id",
                "project_revision",
                "publication_status",
                "runtime_status",
            ])
        );
        let rows = response["locales"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["locale"], "de");
        assert_eq!(rows[0]["text"], long);
        assert_eq!(rows[0]["voice_slot_present"], true);
        assert_eq!(rows[0]["candidate_count"], 1);
        assert_eq!(rows[1]["locale"], "en");
        assert_eq!(rows[1]["voice_slot_present"], false);
        assert_eq!(rows[1]["candidate_count"], 0);
        for row in rows {
            assert_eq!(
                keys(row.as_object().unwrap()),
                BTreeSet::from(["candidate_count", "locale", "text", "voice_slot_present"])
            );
        }
        let backlinks = response["line_backlinks"].as_array().unwrap();
        assert_eq!(backlinks.len(), 1);
        assert_eq!(backlinks[0]["line_id"], id(LINE_TAG).to_string());
        assert_eq!(backlinks[0]["line_revision"], 2);
        assert_eq!(backlinks[0]["display_name"], "Asghan warning line 42");
        assert_eq!(backlinks[0]["speaker_hint"], "Asghan");
        assert_eq!(backlinks[0]["voice_slot_locales"], json!(["de"]));
        assert_eq!(
            keys(backlinks[0].as_object().unwrap()),
            BTreeSet::from([
                "display_name",
                "line_id",
                "line_revision",
                "speaker_hint",
                "voice_slot_locales",
            ])
        );
        assert_eq!(file_tree(store.temp.path()), before);
        assert_eq!(
            fs::read(store.temp.path().join("gore-project.json")).unwrap(),
            store.fixed_head_bytes
        );
    }

    #[test]
    fn seed_read_rejects_wrong_head_id_revision_loc_id_and_origin() {
        let store = published_store(false);
        let before = file_tree(store.temp.path());
        let mut stale_head = store.head.clone();
        stale_head.snapshot.sha256 = Sha256Digest::from_bytes([0x77; 32]);
        let cases = [
            (
                read_wire_with(&store, &stale_head, store.localization_id, 4, LOC_ID),
                "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_HEAD_CONFLICT",
            ),
            (
                read_wire_with(&store, &store.head, id(0x99), 4, LOC_ID),
                "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_NOT_FOUND",
            ),
            (
                read_wire_with(&store, &store.head, store.localization_id, 5, LOC_ID),
                "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REVISION_CONFLICT",
            ),
            (
                read_wire_with(
                    &store,
                    &store.head,
                    store.localization_id,
                    4,
                    "GRD_263_ASGHAN_CHANGED",
                ),
                "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_IDENTITY_CONFLICT",
            ),
        ];
        for (wire, expected) in cases {
            let response = read_revision3_dialog_localization_edit_seed_v1_raw(&wire);
            assert_eq!(error_code(&response), expected, "{response}");
        }

        let imported = publish_project(
            base_project(
                BTreeMap::from([(locale("de"), "Bleib stehen!".to_owned())]),
                imported_origin(LOCALIZATION_TAG),
            ),
            false,
        );
        let response = read_revision3_dialog_localization_edit_seed_v1_raw(&read_wire(&imported));
        assert_eq!(
            error_code(&response),
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_ORIGIN_CONFLICT"
        );
        assert_eq!(file_tree(store.temp.path()), before);
    }

    #[test]
    fn seed_read_rejects_noncanonical_duplicate_extra_and_authority_fields() {
        let store = published_store(false);
        let valid = read_wire(&store);
        let duplicate_revision = valid.replacen(
            r#""expected_localization_revision":4"#,
            r#""expected_localization_revision":4,"expected_localization_revision":4"#,
            1,
        );
        let extra = valid.replacen(r#""root":"#, r#""game_root":"C:/Games/Gothic","root":"#, 1);
        let noncanonical = format!("{valid}\n");
        let wrong_command = valid.replacen(READ_COMMAND, "unknown_localization_command", 1);
        for wire in [duplicate_revision, extra, noncanonical, wrong_command] {
            let response = read_revision3_dialog_localization_edit_seed_v1_raw(&wire);
            assert_eq!(
                error_code(&response),
                "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_INVALID",
                "{response}"
            );
        }

        let duplicate_head = serde_json::to_string(&ExactWireRequest {
            command: READ_COMMAND.to_owned(),
            payload: ReadSeedWirePayload {
                root: store.temp.path().to_string_lossy().into_owned(),
                expected_head_json: serde_json::to_string(&store.head).unwrap().replacen(
                    r#""store_format":1"#,
                    r#""store_format":1,"store_format":1"#,
                    1,
                ),
                localization_id: store.localization_id.to_string(),
                expected_localization_revision: 4,
                expected_loc_id: LOC_ID.to_owned(),
            },
        })
        .unwrap();
        let response = read_revision3_dialog_localization_edit_seed_v1_raw(&duplicate_head);
        assert_eq!(
            error_code(&response),
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_HEAD_INVALID"
        );
    }

    #[test]
    fn seed_text_count_per_text_total_backlink_response_and_signed_limits_are_closed() {
        let too_long = publish_project(
            base_project(
                BTreeMap::from([(
                    locale("de"),
                    "x".repeat(MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_BYTES_V1 + 1),
                )]),
                new_origin(LOC_ID),
            ),
            false,
        );
        let response = read_revision3_dialog_localization_edit_seed_v1_raw(&read_wire(&too_long));
        assert_eq!(
            error_code(&response),
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_TEXT_LIMIT"
        );

        let too_many_texts = (0..=MAX_REVISION3_DIALOG_LOCALIZATION_TEXTS_V1)
            .map(|index| (locale(&format!("aa-{index}")), "x".to_owned()))
            .collect::<BTreeMap<_, _>>();
        let too_many = publish_project(base_project(too_many_texts, new_origin(LOC_ID)), false);
        let response = read_revision3_dialog_localization_edit_seed_v1_raw(&read_wire(&too_many));
        assert_eq!(
            error_code(&response),
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_TEXT_LIMIT"
        );

        let too_large_total = (0..9)
            .map(|index| (locale(&format!("aa-{index}")), "x".repeat(60 * 1024)))
            .collect::<BTreeMap<_, _>>();
        let too_large_total =
            publish_project(base_project(too_large_total, new_origin(LOC_ID)), false);
        let response =
            read_revision3_dialog_localization_edit_seed_v1_raw(&read_wire(&too_large_total));
        assert_eq!(
            error_code(&response),
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_TEXT_LIMIT"
        );

        let mut backlink_project = base_project(
            BTreeMap::from([(locale("de"), "Bleib stehen!".to_owned())]),
            new_origin(LOC_ID),
        );
        for ordinal in 0..=MAX_BACKLINKS {
            let tag = u8::try_from(ordinal % 200 + 50).unwrap();
            // Entity IDs must be unique, so use a full 128-bit ordinal instead of the helper tag.
            let line_id = EntityId::from_bytes((ordinal as u128 + 0x1000).to_be_bytes());
            backlink_project.entities.insert(
                line_id,
                Revision3Entity {
                    id: line_id,
                    display_name: format!("Backlink {ordinal}"),
                    origin: imported_origin(tag),
                    revision: 1,
                    payload: Revision3EntityPayload::DialogLine(DialogLine {
                        localization: TypedRef::new(
                            project_id(),
                            id(LOCALIZATION_TAG),
                            Revision3EntityKind::LocalizationEntry,
                        ),
                        speaker_hint: None,
                        voice_slots: BTreeMap::new(),
                    }),
                },
            );
        }
        let too_many_backlinks = publish_project(backlink_project, false);
        let response =
            read_revision3_dialog_localization_edit_seed_v1_raw(&read_wire(&too_many_backlinks));
        assert_eq!(
            error_code(&response),
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_BACKLINK_LIMIT"
        );

        let ordinary = published_store(false);
        let response = read_seed_inner_with_seam_and_limit(&read_wire(&ordinary), || {}, 64)
            .unwrap_err()
            .response();
        assert_eq!(
            error_code(&response),
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_RESPONSE_LIMIT"
        );
        let signed = read_wire_with(
            &ordinary,
            &ordinary.head,
            ordinary.localization_id,
            i64::MAX as u64 + 1,
            LOC_ID,
        );
        let response = read_revision3_dialog_localization_edit_seed_v1_raw(&signed);
        assert_eq!(
            error_code(&response),
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_SIGNED_WIRE_LIMIT"
        );

        let oversized = "x".repeat(MAX_READ_WIRE_BYTES + 1);
        let response = read_revision3_dialog_localization_edit_seed_v1_raw(&oversized);
        assert_eq!(
            error_code(&response),
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_INPUT_LIMIT"
        );
    }

    #[test]
    fn seed_second_full_open_rejects_concurrent_fixed_head_change() {
        let store = published_store(false);
        let wire = read_wire(&store);
        let root = store.temp.path().to_owned();
        let basis_head = store.head.clone();
        let basis_project = store.project.clone();

        let failure = read_seed_inner_with_seam_and_limit(
            &wire,
            || publish_concurrent_change(&root, &basis_head, &basis_project),
            MAX_READ_RESPONSE_BYTES,
        )
        .unwrap_err();

        assert_eq!(
            failure.code,
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_HEAD_CONFLICT"
        );
    }

    #[test]
    fn prepare_success_reopens_exact_candidate_and_never_publishes_fixed_head() {
        let store = published_store(false);
        let replacement = BTreeMap::from([
            (locale("de"), "Bleib sofort stehen!".to_owned()),
            (locale("fr"), "Halte-là!".to_owned()),
        ]);
        let wire = prepare_wire(&store, replacement.clone());

        // The dispatcher call proves both command registration and the closed raw route.
        let response = crate::dispatch(&wire);

        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["outcome"], "prepared_unpublished");
        assert_eq!(
            response["basis_head_json"],
            serde_json::to_string(&store.head).unwrap()
        );
        assert_ne!(response["head_json"], response["basis_head_json"]);
        assert_eq!(response["project_id"], project_id().to_string());
        assert_eq!(response["revision"], 8);
        assert_eq!(
            response["localization_id"],
            store.localization_id.to_string()
        );
        assert_eq!(response["localization_revision"], 5);
        assert_eq!(response["added_locales"], json!(["fr"]));
        assert_eq!(response["removed_locales"], json!(["en"]));
        assert_eq!(response["build_status"], "blocked");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["topic_authority"], "not_granted");
        assert_eq!(response["publication_status"], "not_supported");
        assert_eq!(
            keys(response.as_object().unwrap()),
            BTreeSet::from([
                "added_locales",
                "basis_head_json",
                "build_status",
                "head_json",
                "localization_id",
                "localization_revision",
                "ok",
                "outcome",
                "project_id",
                "project_json",
                "publication_status",
                "removed_locales",
                "revision",
                "runtime_status",
                "topic_authority",
            ])
        );

        let candidate_json = response["project_json"].as_str().unwrap();
        let candidate = ProjectRevision3::from_json(candidate_json).unwrap();
        let Revision3EntityPayload::LocalizationEntry(localization) =
            &candidate.entities[&store.localization_id].payload
        else {
            panic!("candidate target changed kind")
        };
        assert_eq!(localization.texts, replacement);
        assert_eq!(candidate.entities[&store.localization_id].revision, 5);
        assert!(candidate.authoring_locales.contains(&locale("fr")));
        let checkpoint_head = response["head_json"].as_str().unwrap();
        let reopened = WorkingProjectStore::open_existing(store.temp.path(), ffi_store_limits())
            .unwrap()
            .open_revision3_head_bytes(checkpoint_head.as_bytes(), AssetVerification::Full)
            .unwrap();
        assert_eq!(reopened.project, candidate);
        assert_eq!(
            fs::read(store.temp.path().join("gore-project.json")).unwrap(),
            store.fixed_head_bytes
        );
        let current = WorkingProjectStore::open_existing(store.temp.path(), ffi_store_limits())
            .unwrap()
            .open_current_revision3(AssetVerification::Full)
            .unwrap();
        assert_eq!(current.head, store.head);
        assert_eq!(current.project, store.project);
    }

    #[test]
    fn prepare_rejects_wrong_identity_revision_origin_noop_and_invalid_texts() {
        let store = published_store(false);
        let changed = BTreeMap::from([
            (locale("de"), "Neu".to_owned()),
            (locale("en"), "New".to_owned()),
        ]);
        let mut wrong_id = edit_request(&store, changed.clone());
        wrong_id.localization_id = id(0x99);
        let mut wrong_revision = edit_request(&store, changed.clone());
        wrong_revision.expected_localization_revision = 5;
        let mut wrong_loc_id = edit_request(&store, changed.clone());
        wrong_loc_id.expected_loc_id = "GRD_263_ASGHAN_CHANGED".to_owned();
        let cases = [
            (
                prepare_wire_with_json(&store, &wrong_id.to_canonical_json().unwrap()),
                "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_NOT_FOUND",
            ),
            (
                prepare_wire_with_json(&store, &wrong_revision.to_canonical_json().unwrap()),
                "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REVISION_CONFLICT",
            ),
            (
                prepare_wire_with_json(&store, &wrong_loc_id.to_canonical_json().unwrap()),
                "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_IDENTITY_CONFLICT",
            ),
            (
                prepare_wire(
                    &store,
                    BTreeMap::from([
                        (locale("de"), "Bleib stehen!".to_owned()),
                        (locale("en"), "Stop right there!".to_owned()),
                    ]),
                ),
                "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_NO_CHANGES",
            ),
            (
                prepare_wire(
                    &store,
                    BTreeMap::from([
                        (locale("de"), " \t".to_owned()),
                        (locale("en"), "\n".to_owned()),
                    ]),
                ),
                "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_REJECTED",
            ),
        ];
        for (wire, expected) in cases {
            let response = prepare_revision3_dialog_localization_edit_v1_raw(&wire);
            assert_eq!(error_code(&response), expected, "{response}");
        }

        let imported = publish_project(
            base_project(
                BTreeMap::from([(locale("de"), "Bleib stehen!".to_owned())]),
                imported_origin(LOCALIZATION_TAG),
            ),
            false,
        );
        let response = prepare_revision3_dialog_localization_edit_v1_raw(&prepare_wire(
            &imported,
            BTreeMap::from([(locale("de"), "Neu".to_owned())]),
        ));
        assert_eq!(
            error_code(&response),
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_ORIGIN_CONFLICT"
        );
    }

    #[test]
    fn prepare_voice_slot_locale_and_candidate_text_conflicts_fail_closed() {
        let store = published_store(true);

        let remove_slot_locale = prepare_wire(
            &store,
            BTreeMap::from([(locale("en"), "Stop now!".to_owned())]),
        );
        let response = prepare_revision3_dialog_localization_edit_v1_raw(&remove_slot_locale);
        assert_eq!(
            error_code(&response),
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_VOICE_CONFLICT",
            "{response}"
        );

        let change_candidate_text = prepare_wire(
            &store,
            BTreeMap::from([
                (locale("de"), "Ein anderer Take-Text".to_owned()),
                (locale("en"), "Stop right there!".to_owned()),
            ]),
        );
        let response = prepare_revision3_dialog_localization_edit_v1_raw(&change_candidate_text);
        assert_eq!(
            error_code(&response),
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_VOICE_CONFLICT",
            "{response}"
        );

        // A different locale remains editable while the candidate-bearing text stays exact.
        let allowed = prepare_wire(
            &store,
            BTreeMap::from([
                (locale("de"), "Bleib stehen!".to_owned()),
                (locale("en"), "Stop now!".to_owned()),
            ]),
        );
        let response = prepare_revision3_dialog_localization_edit_v1_raw(&allowed);
        assert_eq!(response["ok"], true, "{response}");
    }

    #[test]
    fn prepare_outer_and_nested_json_are_exact_bounded_and_authority_closed() {
        let store = published_store(false);
        let request = edit_request(
            &store,
            BTreeMap::from([
                (locale("de"), "Neu".to_owned()),
                (locale("en"), "New".to_owned()),
            ]),
        )
        .to_canonical_json()
        .unwrap();
        let valid = prepare_wire_with_json(&store, &request);
        let noncanonical = format!(" {valid}");
        let extra = valid.replacen(r#""root":"#, r#""game_root":"C:/Games/Gothic","root":"#, 1);
        let duplicate_outer = valid.replacen(
            r#""current_project_json":"#,
            r#""current_project_json":"{}","current_project_json":"#,
            1,
        );
        for wire in [noncanonical, extra, duplicate_outer] {
            let response = prepare_revision3_dialog_localization_edit_v1_raw(&wire);
            assert_eq!(
                error_code(&response),
                "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_INVALID",
                "{response}"
            );
        }

        let duplicate_nested = request.replacen(
            r#""expected_revision":7"#,
            r#""expected_revision":7,"expected_revision":7"#,
            1,
        );
        let response = prepare_revision3_dialog_localization_edit_v1_raw(&prepare_wire_with_json(
            &store,
            &duplicate_nested,
        ));
        assert_eq!(
            error_code(&response),
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_INVALID"
        );

        let noncanonical_project = serde_json::to_string_pretty(&store.project).unwrap();
        let wire = serde_json::to_string(&ExactWireRequest {
            command: PREPARE_COMMAND.to_owned(),
            payload: PrepareEditWirePayload {
                current_project_json: noncanonical_project,
                localization_edit_request_json: request,
                root: store.temp.path().to_string_lossy().into_owned(),
            },
        })
        .unwrap();
        let response = prepare_revision3_dialog_localization_edit_v1_raw(&wire);
        assert_eq!(
            error_code(&response),
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_PROJECT_CONFLICT"
        );

        let oversized = "x".repeat(MAX_PREPARE_WIRE_BYTES + 1);
        let response = prepare_revision3_dialog_localization_edit_v1_raw(&oversized);
        assert_eq!(
            error_code(&response),
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_INPUT_LIMIT"
        );
    }

    #[test]
    fn prepare_response_budget_and_both_fixed_head_race_guards_fail_closed() {
        let replacement = || {
            BTreeMap::from([
                (locale("de"), "Bleib sofort stehen!".to_owned()),
                (locale("en"), "Stop now!".to_owned()),
            ])
        };

        let limited = published_store(false);
        let failure = prepare_edit_inner_with_test_seams(
            &prepare_wire(&limited, replacement()),
            || {},
            || {},
            64,
        )
        .unwrap_err();
        assert_eq!(
            failure.code,
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_RESPONSE_LIMIT"
        );
        assert_eq!(
            fs::read(limited.temp.path().join("gore-project.json")).unwrap(),
            limited.fixed_head_bytes
        );

        let after_prepare = published_store(false);
        let root = after_prepare.temp.path().to_owned();
        let basis_head = after_prepare.head.clone();
        let basis_project = after_prepare.project.clone();
        let failure = prepare_edit_inner_with_post_prepare_guard(
            &prepare_wire(&after_prepare, replacement()),
            || publish_concurrent_change(&root, &basis_head, &basis_project),
        )
        .unwrap_err();
        assert_eq!(
            failure.code,
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_HEAD_CONFLICT"
        );

        let final_guard = published_store(false);
        let root = final_guard.temp.path().to_owned();
        let basis_head = final_guard.head.clone();
        let basis_project = final_guard.project.clone();
        let failure =
            prepare_edit_inner_with_final_guard(&prepare_wire(&final_guard, replacement()), || {
                publish_concurrent_change(&root, &basis_head, &basis_project)
            })
            .unwrap_err();
        assert_eq!(
            failure.code,
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_HEAD_CONFLICT"
        );
    }

    #[test]
    fn error_messages_remain_utf8_and_bounded() {
        let failure = Failure::new(
            "AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_INVARIANT",
            "😀".repeat(MAX_ERROR_MESSAGE_BYTES),
        );
        assert!(failure.message.len() <= MAX_ERROR_MESSAGE_BYTES);
        assert!(std::str::from_utf8(failure.message.as_bytes()).is_ok());
        assert!(failure.message.ends_with("..."));
    }
}
