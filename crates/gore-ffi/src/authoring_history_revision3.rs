//! Exact-current authenticated history and prepare-only restore for managed revision-3 projects.
//!
//! Both commands accept complete canonical working heads. Listing returns the bounded history
//! authenticated by the exact current snapshot; unrelated immutable Store objects are invisible.
//! Restore prepares and fully reopens a new `current + 1` checkpoint whose direct parent remains
//! the current head. It never moves or publishes the fixed head and never touches a game or save.

use std::collections::BTreeSet;
use std::path::Path;

use gore_authoring::{
    AssetVerification, PreparedRevision3HistoryRestoreV1, ProjectRevision3,
    Revision3CheckpointPreparation, Revision3HistoryEntryV1, Revision3HistoryErrorV1,
    Revision3HistoryV1, WorkingHead, WorkingProjectStore, WorkingStoreError, WorkingStoreLimits,
    MAX_PROJECT_JSON_BYTES, MAX_REVISION3_HISTORY_PARENT_RECORDS_V1,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::err;

pub(super) const LIST_COMMAND: &str = "authoring_store_list_revision3_history_v1";
pub(super) const RESTORE_COMMAND: &str = "authoring_store_prepare_revision3_history_restore_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_WIRE_BYTES: usize = MAX_PATH_BYTES * 6 + MAX_HEAD_JSON_BYTES * 4 + 8 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

/// Declaration order is the required canonical payload order.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ListHistoryWirePayload {
    expected_head_json: String,
    root: String,
}

/// Declaration order is the required canonical payload order.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PrepareHistoryRestoreWirePayload {
    expected_head_json: String,
    root: String,
    target_head_json: String,
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

pub(super) fn list_revision3_history_v1_raw(input: &str) -> Value {
    list_revision3_history_v1_inner(input).unwrap_or_else(Failure::response)
}

fn list_revision3_history_v1_inner(input: &str) -> Result<Value, Failure> {
    let payload: ListHistoryWirePayload = parse_exact_wire(input, LIST_COMMAND)?;
    validate_path(&payload.root)?;
    let expected_head = parse_canonical_head(&payload.expected_head_json, "expected_head_json")?;
    let store = WorkingProjectStore::open_existing(Path::new(&payload.root), ffi_store_limits())
        .map_err(map_store_error)?;
    let before = exact_current(&store, &expected_head)?;
    let history = store
        .list_current_revision3_history_v1(&expected_head)
        .map_err(map_history_error)?;
    verify_history(&history, &before.project)?;
    exact_current_matches(&store, &expected_head, &before.project)?;

    let mut entries = Vec::with_capacity(history.parents.len() + 1);
    entries.push(history_entry_json(&history.current, true)?);
    for parent in &history.parents {
        entries.push(history_entry_json(parent, false)?);
    }
    let response = json!({
        "ok": true,
        "outcome": "listed_exact_current",
        "basis_head_json": canonical_head_json(&history.basis_head)?,
        "project_id": history.current.project_id.to_string(),
        "project_revision": history.current.project_revision,
        "entries": entries,
        "history_truncated": history.history_truncated,
        "history_authority": "authenticated_bounded_history",
        "project_mutation": "not_performed",
        "game_mutation": "not_performed",
        "save_mutation": "not_performed",
        "build_status": "not_performed",
        "deployment_status": "not_performed",
        "runtime_status": "runtime_unqualified",
        "publication_status": "not_applicable",
    });
    enforce_response_budget(response)
}

pub(super) fn prepare_revision3_history_restore_v1_raw(input: &str) -> Value {
    prepare_revision3_history_restore_v1_inner(input).unwrap_or_else(Failure::response)
}

fn prepare_revision3_history_restore_v1_inner(input: &str) -> Result<Value, Failure> {
    let payload: PrepareHistoryRestoreWirePayload = parse_exact_wire(input, RESTORE_COMMAND)?;
    validate_path(&payload.root)?;
    let expected_head = parse_canonical_head(&payload.expected_head_json, "expected_head_json")?;
    let target_head = parse_canonical_head(&payload.target_head_json, "target_head_json")?;
    if expected_head == target_head {
        return Err(Failure::new(
            "AUTHORING_REVISION3_HISTORY_TARGET_NOT_REACHABLE",
            "the current checkpoint is not an earlier restore target",
        ));
    }

    let store = WorkingProjectStore::open_existing(Path::new(&payload.root), ffi_store_limits())
        .map_err(map_store_error)?;
    let before = exact_current(&store, &expected_head)?;
    let prepared = store
        .prepare_revision3_history_restore_v1(&expected_head, &target_head)
        .map_err(map_history_error)?;
    verify_restore(
        &store,
        &expected_head,
        &before.project,
        &target_head,
        &prepared,
    )?;
    exact_current_matches(&store, &expected_head, &before.project)?;

    let project_json = prepared
        .project
        .to_canonical_json()
        .map_err(|_| invariant())?;
    let response = json!({
        "ok": true,
        "outcome": "prepared_restore_unpublished",
        "basis_head_json": canonical_head_json(&prepared.basis_head)?,
        "direct_parent_head_json": canonical_head_json(&prepared.basis_head)?,
        "restored_from_head_json": canonical_head_json(&prepared.restored_from.head)?,
        "head_json": checkpoint_head_json(&prepared.checkpoint)?,
        "project_json": project_json,
        "project_id": prepared.project.project_id.to_string(),
        "previous_project_revision": before.project.revision,
        "revision": prepared.project.revision,
        "restored_from_revision": prepared.restored_from.project_revision,
        "history_authority": "authenticated_bounded_history",
        "project_mutation": "prepared_not_published",
        "game_mutation": "not_performed",
        "save_mutation": "not_performed",
        "build_status": "not_performed",
        "deployment_status": "not_performed",
        "runtime_status": "runtime_unqualified",
        "publication_status": "not_supported",
    });
    enforce_response_budget(response)
}

fn parse_exact_wire<P>(input: &str, expected_command: &str) -> Result<P, Failure>
where
    P: DeserializeOwned + Serialize,
{
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_HISTORY_INPUT_LIMIT",
            format!("history request exceeds the {MAX_WIRE_BYTES}-byte wire limit"),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != expected_command {
        return Err(invalid_request());
    }
    let canonical = serde_json::to_string(&request).map_err(|_| invariant())?;
    if canonical.as_bytes() != input.as_bytes() {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_path(path: &str) -> Result<(), Failure> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\0') {
        return Err(invalid_request());
    }
    Ok(())
}

fn parse_canonical_head(input: &str, field: &'static str) -> Result<WorkingHead, Failure> {
    if input.is_empty() || input.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_HISTORY_HEAD_INVALID",
            format!("{field} is empty or exceeds its bounded transport limit"),
        ));
    }
    let head: WorkingHead = serde_json::from_str(input).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_HISTORY_HEAD_INVALID",
            format!("{field} is not one closed working head"),
        )
    })?;
    let canonical = canonical_head_json(&head)?;
    if canonical != input {
        return Err(Failure::new(
            "AUTHORING_REVISION3_HISTORY_HEAD_INVALID",
            format!("{field} is not duplicate-free canonical JSON"),
        ));
    }
    Ok(head)
}

fn exact_current(
    store: &WorkingProjectStore,
    expected_head: &WorkingHead,
) -> Result<gore_authoring::OpenedRevision3Checkpoint, Failure> {
    let opened = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if opened.head != *expected_head {
        return Err(head_conflict());
    }
    require_signed_serializable(&opened.head)?;
    require_signed_serializable(&opened.project.revision)?;
    Ok(opened)
}

fn exact_current_matches(
    store: &WorkingProjectStore,
    expected_head: &WorkingHead,
    expected_project: &ProjectRevision3,
) -> Result<(), Failure> {
    let opened = exact_current(store, expected_head)?;
    if opened.project != *expected_project {
        return Err(head_conflict());
    }
    Ok(())
}

fn verify_history(
    history: &Revision3HistoryV1,
    current_project: &ProjectRevision3,
) -> Result<(), Failure> {
    if history.basis_head != history.current.head
        || history.current.project_id != current_project.project_id
        || history.current.project_revision != current_project.revision
        || history.current.meta != current_project.meta
        || history.current.target != current_project.target
        || history.parents.len() > MAX_REVISION3_HISTORY_PARENT_RECORDS_V1
    {
        return Err(invariant());
    }
    let mut expected_revision = current_project.revision;
    let mut heads = BTreeSet::from([canonical_head_json(&history.current.head)?]);
    for parent in &history.parents {
        expected_revision = expected_revision.checked_sub(1).ok_or_else(invariant)?;
        if parent.project_id != current_project.project_id
            || parent.project_revision != expected_revision
            || parent.target != current_project.target
            || !heads.insert(canonical_head_json(&parent.head)?)
        {
            return Err(invariant());
        }
    }
    Ok(())
}

fn verify_restore(
    store: &WorkingProjectStore,
    basis_head: &WorkingHead,
    basis: &ProjectRevision3,
    target_head: &WorkingHead,
    prepared: &PreparedRevision3HistoryRestoreV1,
) -> Result<(), Failure> {
    let expected_revision = basis.revision.checked_add(1).ok_or_else(|| {
        Failure::new(
            "AUTHORING_REVISION3_HISTORY_REVISION_LIMIT",
            "the current project revision cannot be incremented",
        )
    })?;
    if prepared.basis_head != *basis_head
        || prepared.restored_from.head != *target_head
        || prepared.project.project_id != basis.project_id
        || prepared.project.target != basis.target
        || prepared.project.revision != expected_revision
        || prepared.restored_from.project_id != basis.project_id
        || prepared.restored_from.target != basis.target
        || prepared.restored_from.project_revision >= basis.revision
        || prepared.checkpoint.head == prepared.basis_head
    {
        return Err(invariant());
    }
    require_signed_serializable(prepared)?;
    let historical = store
        .open_revision3_head_bytes(
            canonical_head_json(target_head)?.as_bytes(),
            AssetVerification::Full,
        )
        .map_err(map_store_error)?;
    let mut expected_project = historical.project;
    expected_project.revision = expected_revision;
    if historical.head != *target_head || prepared.project != expected_project {
        return Err(invariant());
    }
    let reopened = store
        .open_revision3_head_bytes(&prepared.checkpoint.head_bytes, AssetVerification::Full)
        .map_err(map_store_error)?;
    if reopened.head != prepared.checkpoint.head || reopened.project != prepared.project {
        return Err(invariant());
    }
    Ok(())
}

fn history_entry_json(entry: &Revision3HistoryEntryV1, current: bool) -> Result<Value, Failure> {
    require_signed_serializable(&entry.project_revision)?;
    Ok(json!({
        "head_json": canonical_head_json(&entry.head)?,
        "project_id": entry.project_id.to_string(),
        "project_revision": entry.project_revision,
        "current": current,
    }))
}

fn checkpoint_head_json(checkpoint: &Revision3CheckpointPreparation) -> Result<String, Failure> {
    if checkpoint.head_bytes.is_empty() || checkpoint.head_bytes.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_HISTORY_RESPONSE_LIMIT",
            "prepared history head exceeds its response limit",
        ));
    }
    let value = std::str::from_utf8(&checkpoint.head_bytes).map_err(|_| invariant())?;
    let parsed: WorkingHead = serde_json::from_str(value).map_err(|_| invariant())?;
    let canonical = canonical_head_json(&parsed)?;
    if parsed != checkpoint.head || canonical != value {
        return Err(invariant());
    }
    Ok(canonical)
}

fn canonical_head_json(head: &WorkingHead) -> Result<String, Failure> {
    require_signed_serializable(head)?;
    let value = serde_json::to_string(head).map_err(|_| invariant())?;
    if value.is_empty() || value.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_HISTORY_RESPONSE_LIMIT",
            "working head exceeds its response limit",
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
                "AUTHORING_REVISION3_HISTORY_RESPONSE_LIMIT",
                "history contains an integer outside the signed wire range",
            ))
        }
        Value::Array(values) => values.iter().try_for_each(require_signed_json_value),
        Value::Object(values) => values.values().try_for_each(require_signed_json_value),
        _ => Ok(()),
    }
}

fn enforce_response_budget(response: Value) -> Result<Value, Failure> {
    let bytes = serde_json::to_vec(&response).map_err(|_| invariant())?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_HISTORY_RESPONSE_LIMIT",
            "history response exceeds its bounded transport budget",
        ));
    }
    Ok(response)
}

fn ffi_store_limits() -> WorkingStoreLimits {
    WorkingStoreLimits {
        max_referenced_entity_bytes: MAX_PROJECT_JSON_BYTES as u64,
        ..WorkingStoreLimits::default()
    }
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_HISTORY_REQUEST_INVALID",
        "request must be exact canonical JSON with only the documented history fields",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_HISTORY_HEAD_CONFLICT",
        "the published revision-3 head changed or differs from the caller's exact head",
    )
}

fn invariant() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_HISTORY_INVARIANT",
        "the authenticated revision-3 history failed its exact invariant",
    )
}

fn map_history_error(error: Revision3HistoryErrorV1) -> Failure {
    match error {
        Revision3HistoryErrorV1::Store(error) => map_store_error(error),
        Revision3HistoryErrorV1::InvalidLineage(message) => Failure::new(
            "AUTHORING_REVISION3_HISTORY_LINEAGE_INVALID",
            format!("the authenticated parent chain is invalid: {message}"),
        ),
        Revision3HistoryErrorV1::TargetNotReachable { .. } => Failure::new(
            "AUTHORING_REVISION3_HISTORY_TARGET_NOT_REACHABLE",
            "the requested restore checkpoint is not reachable from the current parent chain",
        ),
        Revision3HistoryErrorV1::ProjectRevisionOverflow { .. } => Failure::new(
            "AUTHORING_REVISION3_HISTORY_REVISION_LIMIT",
            "the current project revision cannot be incremented",
        ),
        Revision3HistoryErrorV1::CandidateReopenMismatch => invariant(),
    }
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match &error {
        WorkingStoreError::InvalidLimits(_) => "AUTHORING_REVISION3_HISTORY_STORE_LIMITS_INVALID",
        WorkingStoreError::MissingRoot(_) => "AUTHORING_REVISION3_HISTORY_STORE_ROOT_MISSING",
        WorkingStoreError::UnsafePath { .. } => "AUTHORING_REVISION3_HISTORY_STORE_PATH_UNSAFE",
        WorkingStoreError::LimitExceeded { .. } => "AUTHORING_REVISION3_HISTORY_STORE_LIMIT",
        WorkingStoreError::HeadConflict { .. } => "AUTHORING_REVISION3_HISTORY_HEAD_CONFLICT",
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_HISTORY_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => "AUTHORING_REVISION3_HISTORY_STORE_OBJECT_MISSING",
        WorkingStoreError::SealMismatch { .. } => "AUTHORING_REVISION3_HISTORY_STORE_SEAL_MISMATCH",
        WorkingStoreError::Collision { .. } => "AUTHORING_REVISION3_HISTORY_STORE_COLLISION",
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_HISTORY_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_HISTORY_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_HISTORY_STORE_IO"
        }
    };
    Failure::new(code, "the revision-3 history Store operation failed")
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

    use gore_authoring::{
        AssetStoreIndex, ContentSeal, FormatV2, GameGenerationAnchor, ProjectId, ProjectMeta,
        SchemaRevisionV3, Sha256Digest,
    };
    use tempfile::TempDir;

    use super::*;

    struct HistoryFixture {
        _temp: TempDir,
        root: String,
        heads: Vec<WorkingHead>,
        projects: Vec<ProjectRevision3>,
    }

    fn project(revision: u64) -> ProjectRevision3 {
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([0x31; 16]),
            revision,
            meta: ProjectMeta {
                name: format!("History fixture {revision}"),
                version: format!("0.{revision}.0"),
                author: "GORE tests".to_owned(),
            },
            target: GameGenerationAnchor {
                executable: ContentSeal {
                    byte_len: 4,
                    sha256: Sha256Digest::from_bytes([0x41; 32]),
                },
            },
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn publish_chain(parent_count: usize) -> HistoryFixture {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_string_lossy().into_owned();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let mut heads = Vec::with_capacity(parent_count + 1);
        let mut projects = Vec::with_capacity(parent_count + 1);
        let initial = project(0);
        let prepared = store.prepare_revision3_checkpoint(None, &initial).unwrap();
        fs::write(temp.path().join("gore-project.json"), &prepared.head_bytes).unwrap();
        heads.push(prepared.head);
        projects.push(initial);
        for revision in 1..=parent_count {
            let candidate = project(revision as u64);
            let prepared = store
                .prepare_revision3_checkpoint(heads.last(), &candidate)
                .unwrap();
            fs::write(temp.path().join("gore-project.json"), &prepared.head_bytes).unwrap();
            heads.push(prepared.head);
            projects.push(candidate);
        }
        HistoryFixture {
            _temp: temp,
            root,
            heads,
            projects,
        }
    }

    fn head_json(head: &WorkingHead) -> String {
        serde_json::to_string(head).unwrap()
    }

    fn list_raw(fixture: &HistoryFixture) -> String {
        serde_json::to_string(&ExactWireRequest {
            command: LIST_COMMAND.to_owned(),
            payload: ListHistoryWirePayload {
                expected_head_json: head_json(fixture.heads.last().unwrap()),
                root: fixture.root.clone(),
            },
        })
        .unwrap()
    }

    fn restore_raw(fixture: &HistoryFixture, target: &WorkingHead) -> String {
        serde_json::to_string(&ExactWireRequest {
            command: RESTORE_COMMAND.to_owned(),
            payload: PrepareHistoryRestoreWirePayload {
                expected_head_json: head_json(fixture.heads.last().unwrap()),
                root: fixture.root.clone(),
                target_head_json: head_json(target),
            },
        })
        .unwrap()
    }

    fn fixed_head_bytes(fixture: &HistoryFixture) -> Vec<u8> {
        fs::read(Path::new(&fixture.root).join("gore-project.json")).unwrap()
    }

    fn error_code(value: &Value) -> &str {
        value
            .pointer("/error/code")
            .and_then(Value::as_str)
            .expect("error response has code")
    }

    #[test]
    fn list_is_bounded_newest_first_and_never_changes_the_fixed_head() {
        let current_revision = MAX_REVISION3_HISTORY_PARENT_RECORDS_V1 + 2;
        let fixture = publish_chain(current_revision);
        let before = fixed_head_bytes(&fixture);
        let response = list_revision3_history_v1_raw(&list_raw(&fixture));

        assert_eq!(response["ok"], true);
        assert_eq!(response["outcome"], "listed_exact_current");
        assert_eq!(
            response["basis_head_json"],
            head_json(&fixture.heads[current_revision])
        );
        assert_eq!(response["project_revision"], current_revision as u64);
        assert_eq!(response["history_truncated"], true);
        assert!(response.get("has_more").is_none());
        assert_eq!(
            response["entries"].as_array().unwrap().len(),
            MAX_REVISION3_HISTORY_PARENT_RECORDS_V1 + 1
        );
        let expected_entry_fields =
            BTreeSet::from(["current", "head_json", "project_id", "project_revision"]);
        for entry in response["entries"].as_array().unwrap() {
            let actual_entry_fields = entry
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            assert_eq!(actual_entry_fields, expected_entry_fields);
        }
        assert_eq!(
            response["entries"][0]["project_revision"],
            current_revision as u64
        );
        assert_eq!(response["entries"][0]["current"], true);
        assert_eq!(
            response["entries"][1]["project_revision"],
            (current_revision - 1) as u64
        );
        assert_eq!(response["entries"][1]["current"], false);
        assert_eq!(
            response["entries"][MAX_REVISION3_HISTORY_PARENT_RECORDS_V1]["project_revision"],
            2
        );
        assert_eq!(
            response["history_authority"],
            "authenticated_bounded_history"
        );
        assert_eq!(response["project_mutation"], "not_performed");
        assert_eq!(response["game_mutation"], "not_performed");
        assert_eq!(response["save_mutation"], "not_performed");
        assert_eq!(response["build_status"], "not_performed");
        assert_eq!(response["deployment_status"], "not_performed");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["publication_status"], "not_applicable");
        assert_eq!(fixed_head_bytes(&fixture), before);
    }

    #[test]
    fn orphan_candidate_is_neither_listed_nor_restore_authority() {
        let fixture = publish_chain(2);
        let store =
            WorkingProjectStore::open_existing(Path::new(&fixture.root), ffi_store_limits())
                .unwrap();
        let orphan_project = project(3);
        let orphan = store
            .prepare_revision3_checkpoint(fixture.heads.last(), &orphan_project)
            .unwrap();
        let before = fixed_head_bytes(&fixture);

        let listed = list_revision3_history_v1_raw(&list_raw(&fixture));
        assert_eq!(listed["history_truncated"], false);
        let listed_heads = listed["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["head_json"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert!(!listed_heads.contains(&head_json(&orphan.head).as_str()));

        let rejected =
            prepare_revision3_history_restore_v1_raw(&restore_raw(&fixture, &orphan.head));
        assert_eq!(
            error_code(&rejected),
            "AUTHORING_REVISION3_HISTORY_TARGET_NOT_REACHABLE"
        );
        assert_eq!(fixed_head_bytes(&fixture), before);
    }

    #[test]
    fn restore_prepares_current_plus_one_with_current_direct_parent_without_publishing() {
        let fixture = publish_chain(3);
        let before = fixed_head_bytes(&fixture);
        let response =
            prepare_revision3_history_restore_v1_raw(&restore_raw(&fixture, &fixture.heads[1]));

        assert_eq!(response["ok"], true);
        assert_eq!(response["outcome"], "prepared_restore_unpublished");
        assert_eq!(response["basis_head_json"], head_json(&fixture.heads[3]));
        assert_eq!(
            response["direct_parent_head_json"],
            head_json(&fixture.heads[3])
        );
        assert_eq!(
            response["restored_from_head_json"],
            head_json(&fixture.heads[1])
        );
        assert_eq!(response["previous_project_revision"], 3);
        assert_eq!(response["revision"], 4);
        assert_eq!(response["restored_from_revision"], 1);
        assert_eq!(
            response["history_authority"],
            "authenticated_bounded_history"
        );
        assert_eq!(response["project_mutation"], "prepared_not_published");
        assert_eq!(response["publication_status"], "not_supported");

        let candidate =
            ProjectRevision3::from_json(response["project_json"].as_str().unwrap()).unwrap();
        let mut expected = fixture.projects[1].clone();
        expected.revision = 4;
        assert_eq!(candidate, expected);
        let store =
            WorkingProjectStore::open_existing(Path::new(&fixture.root), ffi_store_limits())
                .unwrap();
        let reopened = store
            .open_revision3_head_bytes(
                response["head_json"].as_str().unwrap().as_bytes(),
                AssetVerification::Full,
            )
            .unwrap();
        assert_eq!(reopened.project, expected);
        assert_eq!(fixed_head_bytes(&fixture), before);
    }

    #[test]
    fn stale_current_and_current_as_target_fail_before_publication() {
        let fixture = publish_chain(2);
        let stale_list = serde_json::to_string(&ExactWireRequest {
            command: LIST_COMMAND.to_owned(),
            payload: ListHistoryWirePayload {
                expected_head_json: head_json(&fixture.heads[1]),
                root: fixture.root.clone(),
            },
        })
        .unwrap();
        assert_eq!(
            error_code(&list_revision3_history_v1_raw(&stale_list)),
            "AUTHORING_REVISION3_HISTORY_HEAD_CONFLICT"
        );

        let current_target = restore_raw(&fixture, fixture.heads.last().unwrap());
        assert_eq!(
            error_code(&prepare_revision3_history_restore_v1_raw(&current_target)),
            "AUTHORING_REVISION3_HISTORY_TARGET_NOT_REACHABLE"
        );
    }

    #[test]
    fn exact_wire_and_head_bounds_fail_closed() {
        let fixture = publish_chain(1);
        let valid = list_raw(&fixture);
        let duplicate = valid.replacen("\"root\":", "\"root\":\"duplicate\",\"root\":", 1);
        let caller_limit = valid.replacen("\"root\":", "\"limit\":1,\"root\":", 1);
        let unknown = valid.replacen("\"root\":", "\"future\":true,\"root\":", 1);
        for malformed in [valid.clone() + "\n", duplicate, caller_limit, unknown] {
            assert_eq!(
                error_code(&list_revision3_history_v1_raw(&malformed)),
                "AUTHORING_REVISION3_HISTORY_REQUEST_INVALID"
            );
        }

        let noncanonical_head = serde_json::to_string(&ExactWireRequest {
            command: LIST_COMMAND.to_owned(),
            payload: ListHistoryWirePayload {
                expected_head_json: format!(" {}", head_json(fixture.heads.last().unwrap())),
                root: fixture.root.clone(),
            },
        })
        .unwrap();
        assert_eq!(
            error_code(&list_revision3_history_v1_raw(&noncanonical_head)),
            "AUTHORING_REVISION3_HISTORY_HEAD_INVALID"
        );
        assert_eq!(
            error_code(&list_revision3_history_v1_raw(
                &"x".repeat(MAX_WIRE_BYTES + 1)
            )),
            "AUTHORING_REVISION3_HISTORY_INPUT_LIMIT"
        );
    }

    #[test]
    fn public_dispatch_exposes_both_strict_history_commands() {
        let fixture = publish_chain(1);
        let list: Value = serde_json::from_str(&crate::execute_json(&list_raw(&fixture))).unwrap();
        assert_eq!(list["ok"], true);
        let restore: Value = serde_json::from_str(&crate::execute_json(&restore_raw(
            &fixture,
            &fixture.heads[0],
        )))
        .unwrap();
        assert_eq!(restore["ok"], true);
    }
}
