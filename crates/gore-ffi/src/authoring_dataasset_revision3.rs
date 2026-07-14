//! Strict, prepare-only revision-3 DataAsset stage transport for Mod Studio.
//!
//! These routes accept only one exact published working-store head. Preparing or removing a
//! stage may install verified immutable CAS objects and returns a fully reopened candidate head,
//! but never replaces the fixed `gore-project.json` head. The only filesystem source accepted by
//! preparation is a PatchReceipt-v2 path; native `gore-asset` code reopens and verifies the whole
//! receipt chain, the live generation, and the game executable before `gore-authoring` receives
//! its opaque, receipt-unforgeable input.

use std::fs;
use std::io;
use std::path::Path;

use gore_asset::dataasset_workflow::{
    read_patch_receipt_v2, validate_game_asset_path, verify_fixed_leaf_stage_input,
    MAX_RECEIPT_BYTES,
};
use gore_authoring::{
    AssetVerification, DataAssetStageConflictV1, DataAssetStageManifestErrorV1,
    PreparedRevision3DataAssetStageRemovalV1, PreparedRevision3DataAssetStageV1,
    Revision3DataAssetStageViewV1, Revision3DataAssetStagingErrorV1, WorkingHead,
    WorkingProjectStore, WorkingStoreError, WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::err;

pub(super) const PREPARE_COMMAND: &str = "authoring_store_prepare_revision3_dataasset_stage_v1";
pub(super) const LIST_COMMAND: &str = "authoring_store_list_revision3_dataasset_stages_v1";
pub(super) const REMOVE_COMMAND: &str =
    "authoring_store_prepare_remove_revision3_dataasset_stage_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_LIST_WIRE_BYTES: usize = MAX_PATH_BYTES * 6 + MAX_HEAD_JSON_BYTES * 2 + 4 * 1024;
const MAX_PREPARE_WIRE_BYTES: usize = MAX_PATH_BYTES * 12 + MAX_HEAD_JSON_BYTES * 2 + 4 * 1024;
const MAX_REMOVE_WIRE_BYTES: usize = MAX_PATH_BYTES * 12 + MAX_HEAD_JSON_BYTES * 2 + 4 * 1024;
const MAX_MUTATION_BASIS_REVISION: u64 = i64::MAX as u64 - 1;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PrepareWirePayload {
    expected_head_json: String,
    patch_receipt_path: String,
    root: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListWirePayload {
    expected_head_json: String,
    root: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RemoveWirePayload {
    expected_head_json: String,
    root: String,
    target_path: String,
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

pub(super) fn prepare_raw(input: &str) -> Value {
    prepare_inner(input).unwrap_or_else(Failure::response)
}

pub(super) fn list_raw(input: &str) -> Value {
    list_inner(input).unwrap_or_else(Failure::response)
}

pub(super) fn prepare_remove_raw(input: &str) -> Value {
    prepare_remove_inner(input).unwrap_or_else(Failure::response)
}

fn prepare_inner(input: &str) -> Result<Value, Failure> {
    let payload: PrepareWirePayload =
        parse_exact_wire(input, PREPARE_COMMAND, MAX_PREPARE_WIRE_BYTES)?;
    validate_filesystem_path(&payload.root)?;
    validate_filesystem_path(&payload.patch_receipt_path)?;
    let expected_head = parse_exact_head(&payload.expected_head_json)?;
    let store = open_store(&payload.root)?;
    require_exact_basis(&store, &expected_head, true)?;
    preflight_patch_receipt_path(Path::new(&payload.patch_receipt_path))?;

    let patch = read_patch_receipt_v2(Path::new(&payload.patch_receipt_path))
        .map_err(|_| verified_input_invalid())?;
    let verified = verify_fixed_leaf_stage_input(patch).map_err(|_| verified_input_invalid())?;
    let prepared = store
        .prepare_revision3_dataasset_stage_v1(&expected_head, verified)
        .map_err(map_staging_error)?;
    prepared_response(prepared)
}

fn list_inner(input: &str) -> Result<Value, Failure> {
    let payload: ListWirePayload = parse_exact_wire(input, LIST_COMMAND, MAX_LIST_WIRE_BYTES)?;
    validate_filesystem_path(&payload.root)?;
    let expected_head = parse_exact_head(&payload.expected_head_json)?;
    let store = open_store(&payload.root)?;
    let revision = require_exact_basis(&store, &expected_head, false)?;
    let stages = store
        .list_revision3_dataasset_stages_v1(&expected_head)
        .map_err(map_staging_error)?;
    let stages = stages
        .iter()
        .map(stage_response)
        .collect::<Result<Vec<_>, _>>()?;
    let response = json!({
        "ok": true,
        "outcome": "listed_exact_head",
        "basis_head_json": canonical_head_json(&expected_head)?,
        "revision": revision,
        "stages": stages,
        "build_status": "blocked",
        "runtime_status": "runtime_unqualified",
        "artifact_authority": "not_granted",
        "publication_status": "not_supported",
    });
    enforce_response_budget(&response)?;
    Ok(response)
}

fn prepare_remove_inner(input: &str) -> Result<Value, Failure> {
    let payload: RemoveWirePayload =
        parse_exact_wire(input, REMOVE_COMMAND, MAX_REMOVE_WIRE_BYTES)?;
    validate_filesystem_path(&payload.root)?;
    validate_target_path_wire(&payload.target_path)?;
    let expected_head = parse_exact_head(&payload.expected_head_json)?;
    let store = open_store(&payload.root)?;
    require_exact_basis(&store, &expected_head, true)?;
    let prepared = store
        .prepare_remove_revision3_dataasset_stage_v1(&expected_head, &payload.target_path)
        .map_err(map_staging_error)?;
    removal_response(prepared)
}

fn parse_exact_wire<P: DeserializeOwned>(
    input: &str,
    expected_command: &'static str,
    max_bytes: usize,
) -> Result<P, Failure> {
    if input.len() > max_bytes {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_INPUT_LIMIT",
            format!("revision-3 DataAsset request exceeds the {max_bytes}-byte wire limit"),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != expected_command {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_filesystem_path(path: &str) -> Result<(), Failure> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\0') {
        return Err(invalid_request());
    }
    Ok(())
}

fn validate_target_path_wire(path: &str) -> Result<(), Failure> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\0') {
        return Err(invalid_request());
    }
    validate_game_asset_path(path, "DATAASSET_FFI_REMOVE_TARGET").map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_DATAASSET_PROJECT_INVALID",
            "target_path is not a canonical supported /Game asset path",
        )
    })
}

fn parse_exact_head(head_json: &str) -> Result<WorkingHead, Failure> {
    if head_json.is_empty() || head_json.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_HEAD_LIMIT",
            format!("expected_head_json must be 1..={MAX_HEAD_JSON_BYTES} UTF-8 bytes"),
        ));
    }
    let head: WorkingHead = serde_json::from_str(head_json).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_DATAASSET_HEAD_INVALID",
            "expected_head_json is not a valid closed working-store head",
        )
    })?;
    let canonical = canonical_head_json(&head)?;
    if canonical != head_json {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_HEAD_NONCANONICAL",
            "expected_head_json is not in exact canonical form",
        ));
    }
    if head.snapshot.byte_len == 0
        || head.snapshot.byte_len > WorkingStoreLimits::default().max_snapshot_bytes as u64
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_HEAD_INVALID",
            "expected_head_json has an unsupported snapshot size",
        ));
    }
    Ok(head)
}

fn canonical_head_json(head: &WorkingHead) -> Result<String, Failure> {
    let value = serde_json::to_string(head).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_DATAASSET_INVARIANT",
            "working-store head could not be serialized",
        )
    })?;
    if value.is_empty() || value.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_RESPONSE_LIMIT",
            "working-store head exceeds its response limit",
        ));
    }
    Ok(value)
}

fn open_store(root: &str) -> Result<WorkingProjectStore, Failure> {
    WorkingProjectStore::open_existing(Path::new(root), ffi_store_limits()).map_err(map_store_error)
}

fn require_exact_basis(
    store: &WorkingProjectStore,
    expected_head: &WorkingHead,
    will_increment_revision: bool,
) -> Result<u64, Failure> {
    let opened = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if opened.head != *expected_head {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_HEAD_CONFLICT",
            "the published revision-3 head differs from expected_head_json",
        ));
    }
    let limit = if will_increment_revision {
        MAX_MUTATION_BASIS_REVISION
    } else {
        i64::MAX as u64
    };
    if opened.project.revision > limit {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_REVISION_LIMIT",
            format!("the published project revision exceeds the supported {limit} maximum"),
        ));
    }
    require_signed_serializable(&opened.project)?;
    Ok(opened.project.revision)
}

fn prepared_response(prepared: PreparedRevision3DataAssetStageV1) -> Result<Value, Failure> {
    require_signed_serializable(prepared.project())?;
    let head_json = checkpoint_head_json(&prepared.checkpoint().head_bytes)?;
    let response = json!({
        "ok": true,
        "outcome": "prepared_unpublished",
        "basis_head_json": canonical_head_json(prepared.basis_head())?,
        "head_json": head_json,
        "project_json": prepared.canonical_project_json(),
        "revision": prepared.project().revision,
        "stage": stage_response(prepared.stage())?,
        "deduplicated_blobs": prepared.deduplicated_blobs(),
        "build_status": "blocked",
        "runtime_status": "runtime_unqualified",
        "artifact_authority": "not_granted",
        "publication_status": "not_supported",
    });
    enforce_response_budget(&response)?;
    Ok(response)
}

fn removal_response(prepared: PreparedRevision3DataAssetStageRemovalV1) -> Result<Value, Failure> {
    require_signed_serializable(prepared.project())?;
    let head_json = checkpoint_head_json(&prepared.checkpoint().head_bytes)?;
    let response = json!({
        "ok": true,
        "outcome": "prepared_remove_unpublished",
        "basis_head_json": canonical_head_json(prepared.basis_head())?,
        "head_json": head_json,
        "project_json": prepared.canonical_project_json(),
        "revision": prepared.project().revision,
        "removed": stage_response(prepared.removed())?,
        "build_status": "blocked",
        "runtime_status": "runtime_unqualified",
        "artifact_authority": "not_granted",
        "publication_status": "not_supported",
    });
    enforce_response_budget(&response)?;
    Ok(response)
}

fn checkpoint_head_json(head_bytes: &[u8]) -> Result<String, Failure> {
    if head_bytes.is_empty() || head_bytes.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_RESPONSE_LIMIT",
            "prepared working-store head exceeds its response limit",
        ));
    }
    let value = std::str::from_utf8(head_bytes).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_DATAASSET_INVARIANT",
            "prepared working-store head is not UTF-8 JSON",
        )
    })?;
    // Reparse and compare so the wire never promotes noncanonical bytes even if an upstream
    // implementation regresses its already-verified checkpoint contract.
    let parsed = parse_exact_head(value)?;
    canonical_head_json(&parsed)
}

fn stage_response(stage: &Revision3DataAssetStageViewV1) -> Result<Value, Failure> {
    let manifest = serde_json::to_value(stage.manifest()).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_DATAASSET_INVARIANT",
            "closed DataAsset stage manifest could not be serialized",
        )
    })?;
    require_signed_json_value(&manifest)?;
    Ok(json!({
        "manifest_asset": stage.manifest_asset(),
        "manifest": manifest,
    }))
}

fn require_signed_serializable<T: Serialize>(value: &T) -> Result<(), Failure> {
    let value = serde_json::to_value(value).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_DATAASSET_INVARIANT",
            "revision-3 DataAsset value could not be checked for wire safety",
        )
    })?;
    require_signed_json_value(&value)
}

fn require_signed_json_value(value: &Value) -> Result<(), Failure> {
    let unsafe_number = match value {
        Value::Number(number) => number
            .as_u64()
            .is_some_and(|number| number > i64::MAX as u64),
        Value::Array(values) => values
            .iter()
            .any(|value| require_signed_json_value(value).is_err()),
        Value::Object(values) => values
            .values()
            .any(|value| require_signed_json_value(value).is_err()),
        Value::Null | Value::Bool(_) | Value::String(_) => false,
    };
    if unsafe_number {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_RESPONSE_LIMIT",
            "revision-3 DataAsset content contains an integer outside the signed wire range",
        ));
    }
    Ok(())
}

fn enforce_response_budget(response: &Value) -> Result<(), Failure> {
    let encoded = serde_json::to_vec(response).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_DATAASSET_INVARIANT",
            "revision-3 DataAsset response could not be serialized",
        )
    })?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_RESPONSE_LIMIT",
            "revision-3 DataAsset response exceeds its bounded transport budget",
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
        "AUTHORING_REVISION3_DATAASSET_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and only the command-specific fields",
    )
}

fn preflight_patch_receipt_path(path: &Path) -> Result<(), Failure> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(Failure::new(
                "AUTHORING_REVISION3_DATAASSET_INPUT_MISSING",
                "a required DataAsset input does not exist",
            ));
        }
        Err(_) => return Err(verified_input_invalid()),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata_is_reparse(&metadata) {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_INPUT_UNSAFE",
            "a DataAsset input is not a safe regular file",
        ));
    }
    if metadata.len() > MAX_RECEIPT_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_DATAASSET_INPUT_LIMIT",
            "a verified DataAsset input exceeds its bounded resource limit",
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn metadata_is_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_reparse(_metadata: &fs::Metadata) -> bool {
    false
}

fn verified_input_invalid() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_DATAASSET_INPUT_INVALID",
        "the PatchReceipt-v2 provenance chain could not be verified exactly",
    )
}

fn map_staging_error(error: Revision3DataAssetStagingErrorV1) -> Failure {
    match error {
        Revision3DataAssetStagingErrorV1::Store(error) => map_store_error(error),
        Revision3DataAssetStagingErrorV1::Manifest(error) => map_manifest_error(error),
        Revision3DataAssetStagingErrorV1::VerifiedInput(_) => verified_input_invalid(),
        Revision3DataAssetStagingErrorV1::Conflict(error) => map_conflict(error),
        Revision3DataAssetStagingErrorV1::ProjectBinding(_) => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_PROJECT_INVALID",
            "the DataAsset stage is not bound to the exact revision-3 project",
        ),
        Revision3DataAssetStagingErrorV1::CandidateReopenMismatch => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_INVARIANT",
            "the prepared DataAsset candidate did not reopen exactly",
        ),
    }
}

fn map_manifest_error(error: DataAssetStageManifestErrorV1) -> Failure {
    match error {
        DataAssetStageManifestErrorV1::InputTooLarge { .. } => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_INPUT_LIMIT",
            "a closed DataAsset stage manifest exceeds its bounded resource limit",
        ),
        _ => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_MANIFEST_INVALID",
            "a closed DataAsset stage manifest is invalid",
        ),
    }
}

fn map_conflict(error: DataAssetStageConflictV1) -> Failure {
    match error {
        DataAssetStageConflictV1::TargetAlreadyStaged { .. } => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_TARGET_EXISTS",
            "a DataAsset stage already exists for the requested target",
        ),
        DataAssetStageConflictV1::TargetNotStaged { .. } => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_TARGET_MISSING",
            "no DataAsset stage exists for the requested target",
        ),
        DataAssetStageConflictV1::ProjectRevisionOverflow => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_REVISION_LIMIT",
            "the revision-3 project revision cannot be incremented",
        ),
        DataAssetStageConflictV1::AssetCapacityExceeded
        | DataAssetStageConflictV1::AssetBytesExceeded
        | DataAssetStageConflictV1::StageBatchBudgetExceeded { .. } => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_PROJECT_LIMIT",
            "the revision-3 project cannot index the requested DataAsset stage",
        ),
        DataAssetStageConflictV1::AssetMetadataCollision { .. }
        | DataAssetStageConflictV1::DuplicateTarget { .. } => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_PROJECT_INVALID",
            "the revision-3 project has conflicting DataAsset stage metadata",
        ),
        DataAssetStageConflictV1::ExecutableTargetMismatch => Failure::new(
            "AUTHORING_REVISION3_DATAASSET_EXECUTABLE_MISMATCH",
            "the verified live executable differs from the revision-3 project target",
        ),
    }
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::InvalidLimits(_) | WorkingStoreError::LimitExceeded { .. } => {
            "AUTHORING_REVISION3_DATAASSET_STORE_LIMIT"
        }
        WorkingStoreError::UnsafePath { .. } => "AUTHORING_REVISION3_DATAASSET_STORE_PATH_UNSAFE",
        WorkingStoreError::HeadConflict { .. } => "AUTHORING_REVISION3_DATAASSET_HEAD_CONFLICT",
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_DATAASSET_HEAD_MISSING",
        WorkingStoreError::MissingRoot(_) => "AUTHORING_REVISION3_DATAASSET_STORE_ROOT_MISSING",
        WorkingStoreError::MissingObject(_) => "AUTHORING_REVISION3_DATAASSET_STORE_OBJECT_MISSING",
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_DATAASSET_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => "AUTHORING_REVISION3_DATAASSET_STORE_COLLISION",
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_DATAASSET_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_) => "AUTHORING_REVISION3_DATAASSET_STORE_INVARIANT",
        WorkingStoreError::InvalidOgg(_) | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_DATAASSET_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_DATAASSET_STORE_IO"
        }
    };
    Failure::new(code, "the revision-3 working-store operation failed")
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
    use std::fs;
    use std::path::PathBuf;

    use gore_authoring::{ProjectRevision3, WorkingProjectStore};
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;
    use crate::execute_json;

    struct PublishedStore {
        _temp: TempDir,
        root: PathBuf,
        head: WorkingHead,
        head_json: String,
        fixed_head_bytes: Vec<u8>,
    }

    fn revision3_project(
        revision: u64,
        executable_len: u64,
        executable_sha256: &str,
    ) -> ProjectRevision3 {
        serde_json::from_value(json!({
            "format": 2,
            "schema_revision": 3,
            "project_id": "dada0303030303030303030303030303",
            "revision": revision,
            "meta": {"name": "DataAsset FFI", "version": "1.0.0", "author": "tests"},
            "target": {"executable": {
                "byte_len": executable_len,
                "sha256": executable_sha256
            }},
            "authoring_locales": [],
            "entities": {},
            "asset_store": {"assets": {}}
        }))
        .unwrap()
    }

    fn published_store(revision: u64) -> PublishedStore {
        published_project(revision3_project(
            revision,
            123,
            "4545454545454545454545454545454545454545454545454545454545454545",
        ))
    }

    fn published_project(project: ProjectRevision3) -> PublishedStore {
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let prepared = store.prepare_revision3_checkpoint(None, &project).unwrap();
        fs::write(temp.path().join("gore-project.json"), &prepared.head_bytes).unwrap();
        PublishedStore {
            root: temp.path().to_path_buf(),
            head_json: serde_json::to_string(&prepared.head).unwrap(),
            head: prepared.head,
            fixed_head_bytes: prepared.head_bytes,
            _temp: temp,
        }
    }

    fn raw(command: &str, payload: Value) -> String {
        json!({"command": command, "payload": payload}).to_string()
    }

    fn call(command: &str, payload: Value) -> Value {
        serde_json::from_str(&execute_json(&raw(command, payload))).unwrap()
    }

    fn assert_fixed_head_unchanged(store: &PublishedStore) {
        assert_eq!(
            fs::read(store.root.join("gore-project.json")).unwrap(),
            store.fixed_head_bytes
        );
    }

    #[test]
    fn list_empty_exact_head_is_closed_and_read_only() {
        let store = published_store(7);
        let response = call(
            LIST_COMMAND,
            json!({"root": store.root, "expected_head_json": store.head_json}),
        );
        assert_eq!(response["ok"], true);
        assert_eq!(response["outcome"], "listed_exact_head");
        assert_eq!(response["revision"], 7);
        assert_eq!(response["stages"], json!([]));
        assert_eq!(response["build_status"], "blocked");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["artifact_authority"], "not_granted");
        assert_eq!(response["publication_status"], "not_supported");
        assert_eq!(response["basis_head_json"], store.head_json);
        assert_fixed_head_unchanged(&store);
    }

    #[test]
    fn all_routes_reject_unknown_duplicate_missing_and_wrongly_typed_fields() {
        let store = published_store(7);
        let root = serde_json::to_string(&store.root).unwrap();
        let head = serde_json::to_string(&store.head_json).unwrap();
        let missing_receipt = serde_json::to_string(&store.root.join("missing.json")).unwrap();
        let command = |value: &str| serde_json::to_string(value).unwrap();
        let request = |command_name: &str, payload: String| {
            [
                "{\"command\":",
                &command(command_name),
                ",\"payload\":",
                &payload,
                "}",
            ]
            .concat()
        };
        let cases = [
            request(
                LIST_COMMAND,
                format!(r#"{{"expected_head_json":{head},"root":{root},"extra":true}}"#),
            ),
            [
                "{\"command\":",
                &command(LIST_COMMAND),
                ",\"command\":",
                &command(LIST_COMMAND),
                ",\"payload\":",
                &format!(r#"{{"expected_head_json":{head},"root":{root}}}"#),
                "}",
            ]
            .concat(),
            request(
                LIST_COMMAND,
                format!(
                    r#"{{"expected_head_json":{head},"expected_head_json":{head},"root":{root}}}"#
                ),
            ),
            request(LIST_COMMAND, format!(r#"{{"root":{root}}}"#)),
            request(
                LIST_COMMAND,
                format!(r#"{{"expected_head_json":null,"root":{root}}}"#),
            ),
            request(
                PREPARE_COMMAND,
                format!(
                    r#"{{"expected_head_json":{head},"patch_receipt_path":{missing_receipt},"root":{root},"extra":0}}"#
                ),
            ),
            request(
                PREPARE_COMMAND,
                format!(
                    r#"{{"expected_head_json":{head},"patch_receipt_path":false,"root":{root}}}"#
                ),
            ),
            request(
                REMOVE_COMMAND,
                format!(r#"{{"expected_head_json":{head},"root":{root}}}"#),
            ),
            request(
                REMOVE_COMMAND,
                format!(r#"{{"expected_head_json":{head},"root":{root},"target_path":7}}"#),
            ),
        ];
        for input in cases {
            let response: Value = serde_json::from_str(&execute_json(&input)).unwrap();
            assert_eq!(
                response["error"]["code"], "AUTHORING_REVISION3_DATAASSET_REQUEST_INVALID",
                "input was {input}"
            );
        }
        assert_fixed_head_unchanged(&store);
    }

    #[test]
    fn direct_raw_entrypoints_reject_cross_command_wires() {
        let payload = json!({"expected_head_json": "x", "root": "x"});
        let input = raw(PREPARE_COMMAND, payload);
        assert_eq!(
            list_raw(&input)["error"]["code"],
            "AUTHORING_REVISION3_DATAASSET_REQUEST_INVALID"
        );
    }

    #[test]
    fn heads_must_be_present_exact_canonical_and_current() {
        let store = published_store(7);
        let mut stale = store.head.clone();
        stale.snapshot.byte_len += 1;
        let stale = serde_json::to_string(&stale).unwrap();
        let cases = [
            (String::new(), "AUTHORING_REVISION3_DATAASSET_HEAD_LIMIT"),
            (
                format!(" {}", store.head_json),
                "AUTHORING_REVISION3_DATAASSET_HEAD_NONCANONICAL",
            ),
            (
                r#"{"store_format":1}"#.to_owned(),
                "AUTHORING_REVISION3_DATAASSET_HEAD_INVALID",
            ),
            (
                format!(
                    r#"{{"store_format":1,"snapshot":{{"byte_len":{},"byte_len":{},"sha256":"{}"}}}}"#,
                    store.head.snapshot.byte_len,
                    store.head.snapshot.byte_len,
                    store.head.snapshot.sha256
                ),
                "AUTHORING_REVISION3_DATAASSET_HEAD_INVALID",
            ),
            (stale, "AUTHORING_REVISION3_DATAASSET_HEAD_CONFLICT"),
        ];
        for (head, code) in cases {
            let response = call(
                LIST_COMMAND,
                json!({"root": store.root, "expected_head_json": head}),
            );
            assert_eq!(response["error"]["code"], code);
        }
        assert_fixed_head_unchanged(&store);
    }

    #[test]
    fn prepare_missing_or_invalid_receipt_is_sanitized_and_never_publishes() {
        let store = published_store(7);
        let misleading = store.root.join("limit-changed-unsafe");
        fs::create_dir(&misleading).unwrap();
        let missing = misleading.join("secret-receipt-name.json");
        let missing_response = call(
            PREPARE_COMMAND,
            json!({
                "root": store.root,
                "expected_head_json": store.head_json,
                "patch_receipt_path": missing,
            }),
        );
        assert_eq!(
            missing_response["error"]["code"],
            "AUTHORING_REVISION3_DATAASSET_INPUT_MISSING"
        );
        assert!(!missing_response.to_string().contains("secret-receipt-name"));

        let invalid = misleading.join("invalid-receipt.json");
        fs::write(&invalid, br#"{"format":"not-a-patch-receipt"}"#).unwrap();
        let invalid_response = call(
            PREPARE_COMMAND,
            json!({
                "root": store.root,
                "expected_head_json": store.head_json,
                "patch_receipt_path": invalid,
            }),
        );
        assert_eq!(
            invalid_response["error"]["code"],
            "AUTHORING_REVISION3_DATAASSET_INPUT_INVALID"
        );
        assert!(!invalid_response.to_string().contains("invalid-receipt"));

        let oversized = misleading.join("oversized-receipt.json");
        fs::File::create(&oversized)
            .unwrap()
            .set_len(MAX_RECEIPT_BYTES + 1)
            .unwrap();
        let oversized_response = call(
            PREPARE_COMMAND,
            json!({
                "root": store.root,
                "expected_head_json": store.head_json,
                "patch_receipt_path": oversized,
            }),
        );
        assert_eq!(
            oversized_response["error"]["code"],
            "AUTHORING_REVISION3_DATAASSET_INPUT_LIMIT"
        );

        let directory_response = call(
            PREPARE_COMMAND,
            json!({
                "root": store.root,
                "expected_head_json": store.head_json,
                "patch_receipt_path": misleading,
            }),
        );
        assert_eq!(
            directory_response["error"]["code"],
            "AUTHORING_REVISION3_DATAASSET_INPUT_UNSAFE"
        );
        assert_fixed_head_unchanged(&store);
    }

    #[test]
    fn remove_validates_target_and_requires_an_existing_exact_stage() {
        let store = published_store(7);
        for target in ["", "Game/NoSlash", "/Engine/Foreign", "/Game/../Escape"] {
            let response = call(
                REMOVE_COMMAND,
                json!({
                    "root": store.root,
                    "expected_head_json": store.head_json,
                    "target_path": target,
                }),
            );
            let expected = if target.is_empty() {
                "AUTHORING_REVISION3_DATAASSET_REQUEST_INVALID"
            } else {
                "AUTHORING_REVISION3_DATAASSET_PROJECT_INVALID"
            };
            assert_eq!(response["error"]["code"], expected, "target {target:?}");
        }
        let missing = call(
            REMOVE_COMMAND,
            json!({
                "root": store.root,
                "expected_head_json": store.head_json,
                "target_path": "/Game/Data/Missing",
            }),
        );
        assert_eq!(
            missing["error"]["code"],
            "AUTHORING_REVISION3_DATAASSET_TARGET_MISSING"
        );
        assert_fixed_head_unchanged(&store);
    }

    #[test]
    fn stale_head_blocks_prepare_before_receipt_access() {
        let store = published_store(7);
        let mut stale = store.head.clone();
        stale.snapshot.byte_len += 1;
        let marker = store.root.join("receipt-that-must-not-be-read.json");
        let response = call(
            PREPARE_COMMAND,
            json!({
                "root": store.root,
                "expected_head_json": serde_json::to_string(&stale).unwrap(),
                "patch_receipt_path": marker,
            }),
        );
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_DATAASSET_HEAD_CONFLICT"
        );
        assert_fixed_head_unchanged(&store);
    }

    #[test]
    fn paths_and_wire_sizes_are_bounded_before_store_or_receipt_io() {
        let too_long = "x".repeat(MAX_PATH_BYTES + 1);
        for response in [
            list_raw(&raw(
                LIST_COMMAND,
                json!({"root": too_long, "expected_head_json": "x"}),
            )),
            prepare_raw(&raw(
                PREPARE_COMMAND,
                json!({"root": "x", "expected_head_json": "x", "patch_receipt_path": too_long}),
            )),
            prepare_remove_raw(&raw(
                REMOVE_COMMAND,
                json!({"root": "x", "expected_head_json": "x", "target_path": too_long}),
            )),
        ] {
            assert_eq!(
                response["error"]["code"],
                "AUTHORING_REVISION3_DATAASSET_REQUEST_INVALID"
            );
        }
        for response in [
            list_raw(&raw(
                LIST_COMMAND,
                json!({"root": "bad\0root", "expected_head_json": "x"}),
            )),
            prepare_raw(&raw(
                PREPARE_COMMAND,
                json!({"root": "x", "expected_head_json": "x", "patch_receipt_path": "bad\0receipt"}),
            )),
            prepare_remove_raw(&raw(
                REMOVE_COMMAND,
                json!({"root": "x", "expected_head_json": "x", "target_path": "/Game/Bad\0Target"}),
            )),
        ] {
            assert_eq!(
                response["error"]["code"],
                "AUTHORING_REVISION3_DATAASSET_REQUEST_INVALID"
            );
        }
        let response = list_raw(&" ".repeat(MAX_LIST_WIRE_BYTES + 1));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_DATAASSET_INPUT_LIMIT"
        );
    }

    #[test]
    fn mutation_revisions_remain_signed_64_bit_safe() {
        let store = published_store(i64::MAX as u64);
        let list = call(
            LIST_COMMAND,
            json!({"root": store.root, "expected_head_json": store.head_json}),
        );
        assert_eq!(list["ok"], true);
        assert_eq!(list["revision"].as_u64(), Some(i64::MAX as u64));
        let remove = call(
            REMOVE_COMMAND,
            json!({
                "root": store.root,
                "expected_head_json": store.head_json,
                "target_path": "/Game/Data/Anything",
            }),
        );
        assert_eq!(
            remove["error"]["code"],
            "AUTHORING_REVISION3_DATAASSET_REVISION_LIMIT"
        );
        let prepare = call(
            PREPARE_COMMAND,
            json!({
                "root": store.root,
                "expected_head_json": store.head_json,
                "patch_receipt_path": store.root.join("must-not-be-read.json"),
            }),
        );
        assert_eq!(
            prepare["error"]["code"],
            "AUTHORING_REVISION3_DATAASSET_REVISION_LIMIT"
        );
        assert_fixed_head_unchanged(&store);

        let outside_wire = published_store(i64::MAX as u64 + 1);
        let rejected_list = call(
            LIST_COMMAND,
            json!({
                "root": outside_wire.root,
                "expected_head_json": outside_wire.head_json,
            }),
        );
        assert_eq!(
            rejected_list["error"]["code"],
            "AUTHORING_REVISION3_DATAASSET_REVISION_LIMIT"
        );
        assert_fixed_head_unchanged(&outside_wire);
    }

    #[test]
    fn list_rejects_non_revision_unsigned_project_values() {
        let store = published_project(revision3_project(
            7,
            u64::MAX,
            "4545454545454545454545454545454545454545454545454545454545454545",
        ));
        let response = call(
            LIST_COMMAND,
            json!({"root": store.root, "expected_head_json": store.head_json}),
        );
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_DATAASSET_RESPONSE_LIMIT"
        );
        assert_fixed_head_unchanged(&store);

        assert_eq!(
            require_signed_json_value(&json!({
                "nested": [{"main_utoc": {"length": u64::MAX}}]
            }))
            .unwrap_err()
            .code,
            "AUTHORING_REVISION3_DATAASSET_RESPONSE_LIMIT"
        );
    }

    #[test]
    fn response_budget_and_error_messages_fail_closed() {
        let value = json!({"x": "y"});
        assert!(enforce_response_budget(&value).is_ok());
        let oversized = json!({"x": "z".repeat(MAX_RESPONSE_BYTES)});
        assert_eq!(
            enforce_response_budget(&oversized).unwrap_err().code,
            "AUTHORING_REVISION3_DATAASSET_RESPONSE_LIMIT"
        );
        let failure = Failure::new("X", "音".repeat(MAX_ERROR_MESSAGE_BYTES));
        assert!(failure.message.len() <= MAX_ERROR_MESSAGE_BYTES);
        assert!(failure.message.ends_with("..."));
    }

    #[test]
    #[ignore = "requires GORE_DATAASSET_PATCH_RECEIPT bound to an installed live game generation"]
    fn real_verified_prepare_list_remove_are_closed_prepare_only_and_reopen() {
        let patch_path = PathBuf::from(
            std::env::var_os("GORE_DATAASSET_PATCH_RECEIPT")
                .expect("set GORE_DATAASSET_PATCH_RECEIPT"),
        );
        let verified = verify_fixed_leaf_stage_input(
            read_patch_receipt_v2(&patch_path).expect("read real PatchReceipt v2"),
        )
        .expect("verify real fixed-leaf stage input");
        let executable_len = verified.executable_anchor().length();
        let executable_sha256 = verified
            .executable_anchor()
            .sha256()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();

        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let project = revision3_project(0, executable_len, &executable_sha256);
        let basis = store.prepare_revision3_checkpoint(None, &project).unwrap();
        fs::write(temp.path().join("gore-project.json"), &basis.head_bytes).unwrap();
        let fixed_before = basis.head_bytes.clone();
        let head_json = serde_json::to_string(&basis.head).unwrap();

        let response = call(
            PREPARE_COMMAND,
            json!({
                "root": temp.path(),
                "expected_head_json": head_json,
                "patch_receipt_path": patch_path,
            }),
        );
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["outcome"], "prepared_unpublished");
        assert_eq!(response["build_status"], "blocked");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["artifact_authority"], "not_granted");
        assert_eq!(response["publication_status"], "not_supported");
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            fixed_before
        );

        let serialized = response.to_string();
        assert!(!serialized.contains("patch_receipt_path"));
        assert!(!serialized.contains("absolute_offset"));
        assert!(!serialized.contains(&patch_path.display().to_string()));
        let reopened = store
            .open_revision3_head_bytes(
                response["head_json"].as_str().unwrap().as_bytes(),
                AssetVerification::Full,
            )
            .unwrap();
        assert_eq!(reopened.project.revision, 1);

        // Publication is intentionally outside every route. Simulate the managed session's
        // separately guarded fixed-head CAS step only so the remaining exact-head routes can be
        // exercised against the prepared candidate.
        let staged_head_json = response["head_json"].as_str().unwrap().to_owned();
        fs::write(
            temp.path().join("gore-project.json"),
            staged_head_json.as_bytes(),
        )
        .unwrap();
        let staged_fixed_head = staged_head_json.as_bytes().to_vec();
        let listed = call(
            LIST_COMMAND,
            json!({
                "root": temp.path(),
                "expected_head_json": staged_head_json,
            }),
        );
        assert_eq!(listed["ok"], true, "{listed}");
        assert_eq!(listed["stages"].as_array().unwrap().len(), 1);
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            staged_fixed_head
        );

        let target_path = listed["stages"][0]["manifest"]["target_path"]
            .as_str()
            .unwrap()
            .to_owned();
        let removed = call(
            REMOVE_COMMAND,
            json!({
                "root": temp.path(),
                "expected_head_json": staged_head_json,
                "target_path": target_path,
            }),
        );
        assert_eq!(removed["ok"], true, "{removed}");
        assert_eq!(removed["outcome"], "prepared_remove_unpublished");
        assert_eq!(removed["build_status"], "blocked");
        assert_eq!(removed["runtime_status"], "runtime_unqualified");
        assert_eq!(removed["artifact_authority"], "not_granted");
        assert_eq!(removed["publication_status"], "not_supported");
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            staged_fixed_head
        );
        let reopened_removal = store
            .open_revision3_head_bytes(
                removed["head_json"].as_str().unwrap().as_bytes(),
                AssetVerification::Full,
            )
            .unwrap();
        assert_eq!(reopened_removal.project.revision, 2);
    }
}
