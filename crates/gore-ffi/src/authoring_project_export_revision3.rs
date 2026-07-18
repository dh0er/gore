//! Exact-current managed revision-3 snapshot export.
//!
//! The wire deliberately accepts only one Store root, one exact canonical head, and one output
//! path. Native code derives and verifies the complete immutable closure and publishes one
//! no-clobber review artifact. It never mutates the project, game, save, build, or runtime.

use std::path::Path;

use gore_authoring::{
    Revision3ExactSnapshotExportErrorV1, Revision3ExactSnapshotExportPublicationV1,
    Revision3ExactSnapshotExportPublicationV2, WorkingHead, WorkingProjectStore, WorkingStoreError,
    WorkingStoreLimits, REVISION3_EXACT_SNAPSHOT_ARTIFACT_KIND_V1,
    REVISION3_EXACT_SNAPSHOT_ARTIFACT_KIND_V2, REVISION3_EXACT_SNAPSHOT_EXPORT_FORMAT_V1,
    REVISION3_EXACT_SNAPSHOT_EXPORT_FORMAT_V2, REVISION3_EXACT_SNAPSHOT_MANIFEST_FILE_V1,
    REVISION3_EXACT_SNAPSHOT_MANIFEST_FILE_V2, REVISION3_EXACT_SNAPSHOT_RESTORE_STATUS_V1,
    REVISION3_EXACT_SNAPSHOT_RESTORE_STATUS_V2,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_export_revision3_exact_snapshot_v1";
pub(super) const COMMAND_V2: &str = "authoring_store_export_revision3_exact_snapshot_v2";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_WIRE_BYTES: usize = (MAX_PATH_BYTES * 2 + MAX_HEAD_JSON_BYTES) * 6 + 4 * 1024;

const CLEANUP_WARNING_CODE: &str = "AUTHORING_REVISION3_EXPORT_CLEANUP_WARNING";
const CLEANUP_WARNING_MESSAGE: &str =
    "the verified snapshot was published, but private staging cleanup was incomplete";
const UNCERTAIN_WARNING_CODE: &str = "AUTHORING_REVISION3_EXPORT_PUBLICATION_UNCERTAIN";
const UNCERTAIN_WARNING_MESSAGE: &str =
    "publication may have completed; do not retry automatically";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactSnapshotExportWirePayload {
    expected_head_json: String,
    output: String,
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

pub(super) fn export_revision3_exact_snapshot_v1_raw(input: &str) -> Value {
    export_revision3_exact_snapshot_v1_inner(input).unwrap_or_else(Failure::response)
}

pub(super) fn export_revision3_exact_snapshot_v2_raw(input: &str) -> Value {
    export_revision3_exact_snapshot_v2_inner(input).unwrap_or_else(Failure::response)
}

fn export_revision3_exact_snapshot_v1_inner(input: &str) -> Result<Value, Failure> {
    let payload: ExactSnapshotExportWirePayload = parse_exact_wire(input)?;
    validate_path(&payload.root)?;
    validate_path(&payload.output)?;
    let expected_head = parse_canonical_head(&payload.expected_head_json)?;

    let store =
        WorkingProjectStore::open_existing(Path::new(&payload.root), WorkingStoreLimits::default())
            .map_err(map_open_store_error)?;
    let publication = store
        .export_current_revision3_exact_snapshot_v1(&expected_head, Path::new(&payload.output))
        .map_err(map_export_error)?;

    Ok(publication_response(
        publication,
        &expected_head,
        payload.expected_head_json,
        payload.output,
    ))
}

fn export_revision3_exact_snapshot_v2_inner(input: &str) -> Result<Value, Failure> {
    let payload: ExactSnapshotExportWirePayload = parse_exact_wire_v2(input)?;
    validate_path(&payload.root)?;
    validate_path(&payload.output)?;
    let expected_head = parse_canonical_head(&payload.expected_head_json)?;

    let store =
        WorkingProjectStore::open_existing(Path::new(&payload.root), WorkingStoreLimits::default())
            .map_err(map_open_store_error)?;
    let publication = store
        .export_current_revision3_exact_snapshot_v2(&expected_head, Path::new(&payload.output))
        .map_err(map_export_error)?;

    Ok(publication_response_v2(
        publication,
        &expected_head,
        payload.expected_head_json,
        payload.output,
    ))
}

fn publication_response(
    publication: Revision3ExactSnapshotExportPublicationV1,
    expected_head: &WorkingHead,
    basis_head_json: String,
    output: String,
) -> Value {
    // Everything used below is bounded and prepared before native publication. Keep response
    // construction infallible so no ordinary error can be reported after the boundary may have
    // been crossed.
    let receipt_head_matches = publication.receipt().head == *expected_head;
    let (mut outcome, mut publication_status, mut warning, receipt) = match publication {
        Revision3ExactSnapshotExportPublicationV1::Exported(receipt) => {
            ("exported", "published", Value::Null, receipt)
        }
        Revision3ExactSnapshotExportPublicationV1::ExportedWithCleanupWarning(receipt) => (
            "exported_with_cleanup_warning",
            "published_with_cleanup_warning",
            json!({
                "code": CLEANUP_WARNING_CODE,
                "message": CLEANUP_WARNING_MESSAGE,
            }),
            receipt,
        ),
        Revision3ExactSnapshotExportPublicationV1::PublicationUncertain(receipt) => (
            "publication_uncertain",
            "publication_uncertain",
            json!({
                "code": UNCERTAIN_WARNING_CODE,
                "message": UNCERTAIN_WARNING_MESSAGE,
            }),
            receipt,
        ),
    };
    if !receipt_head_matches {
        // Publication may already have completed, so an impossible internal basis mismatch can
        // only become the fail-closed terminal. The managed client must not trust or retry it.
        outcome = "publication_uncertain";
        publication_status = "publication_uncertain";
        warning = json!({
            "code": UNCERTAIN_WARNING_CODE,
            "message": UNCERTAIN_WARNING_MESSAGE,
        });
    }

    json!({
        "ok": true,
        "outcome": outcome,
        "format": REVISION3_EXACT_SNAPSHOT_EXPORT_FORMAT_V1,
        "artifact_kind": REVISION3_EXACT_SNAPSHOT_ARTIFACT_KIND_V1,
        "restore_status": REVISION3_EXACT_SNAPSHOT_RESTORE_STATUS_V1,
        "basis_head_json": basis_head_json,
        "project_id": receipt.project_id.to_string(),
        "project_revision": receipt.project_revision,
        "output": output,
        "archive": receipt.archive,
        "manifest": {
            "relative_name": REVISION3_EXACT_SNAPSHOT_MANIFEST_FILE_V1,
            "byte_len": receipt.manifest.byte_len,
            "sha256": receipt.manifest.sha256,
        },
        "closure": receipt.closure,
        "publication_status": publication_status,
        "retry_safe": false,
        "warning": warning,
        "project_mutation": "not_performed",
        "game_mutation": "not_performed",
        "save_mutation": "not_performed",
        "build_status": "not_performed",
        "deployment_status": "not_performed",
        "runtime_status": "runtime_unqualified",
    })
}

fn publication_response_v2(
    publication: Revision3ExactSnapshotExportPublicationV2,
    expected_head: &WorkingHead,
    basis_head_json: String,
    output: String,
) -> Value {
    let receipt_head_matches = publication.receipt().head == *expected_head;
    let (mut outcome, mut publication_status, mut warning, receipt) = match publication {
        Revision3ExactSnapshotExportPublicationV2::Exported(receipt) => {
            ("exported", "published", Value::Null, receipt)
        }
        Revision3ExactSnapshotExportPublicationV2::ExportedWithCleanupWarning(receipt) => (
            "exported_with_cleanup_warning",
            "published_with_cleanup_warning",
            json!({
                "code": CLEANUP_WARNING_CODE,
                "message": CLEANUP_WARNING_MESSAGE,
            }),
            receipt,
        ),
        Revision3ExactSnapshotExportPublicationV2::PublicationUncertain(receipt) => (
            "publication_uncertain",
            "publication_uncertain",
            json!({
                "code": UNCERTAIN_WARNING_CODE,
                "message": UNCERTAIN_WARNING_MESSAGE,
            }),
            receipt,
        ),
    };
    if !receipt_head_matches {
        outcome = "publication_uncertain";
        publication_status = "publication_uncertain";
        warning = json!({
            "code": UNCERTAIN_WARNING_CODE,
            "message": UNCERTAIN_WARNING_MESSAGE,
        });
    }

    json!({
        "ok": true,
        "outcome": outcome,
        "format": REVISION3_EXACT_SNAPSHOT_EXPORT_FORMAT_V2,
        "artifact_kind": REVISION3_EXACT_SNAPSHOT_ARTIFACT_KIND_V2,
        "restore_status": REVISION3_EXACT_SNAPSHOT_RESTORE_STATUS_V2,
        "basis_head_json": basis_head_json,
        "project_id": receipt.project_id.to_string(),
        "project_revision": receipt.project_revision,
        "output": output,
        "archive": receipt.archive,
        "manifest": {
            "relative_name": REVISION3_EXACT_SNAPSHOT_MANIFEST_FILE_V2,
            "byte_len": receipt.manifest.byte_len,
            "sha256": receipt.manifest.sha256,
        },
        "closure": receipt.closure,
        "publication_status": publication_status,
        "retry_safe": false,
        "warning": warning,
        "project_mutation": "not_performed",
        "game_mutation": "not_performed",
        "save_mutation": "not_performed",
        "build_status": "not_performed",
        "deployment_status": "not_performed",
        "runtime_status": "runtime_unqualified",
    })
}

fn parse_exact_wire<P: DeserializeOwned>(input: &str) -> Result<P, Failure> {
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_EXPORT_INPUT_LIMIT",
            format!("managed snapshot export request exceeds the {MAX_WIRE_BYTES}-byte limit"),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn parse_exact_wire_v2<P: DeserializeOwned>(input: &str) -> Result<P, Failure> {
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_EXPORT_INPUT_LIMIT",
            format!("managed snapshot export request exceeds the {MAX_WIRE_BYTES}-byte limit"),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND_V2 {
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

fn parse_canonical_head(input: &str) -> Result<WorkingHead, Failure> {
    if input.is_empty() || input.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_EXPORT_HEAD_INVALID",
            "expected_head_json is empty or exceeds its bounded transport limit",
        ));
    }
    let head: WorkingHead = serde_json::from_str(input).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_EXPORT_HEAD_INVALID",
            "expected_head_json is not one closed revision-3 working head",
        )
    })?;
    let canonical = serde_json::to_string(&head).map_err(|_| invariant_failure())?;
    if canonical != input
        || head.snapshot.byte_len == 0
        || head.snapshot.byte_len > WorkingStoreLimits::default().max_snapshot_bytes as u64
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_EXPORT_HEAD_INVALID",
            "expected_head_json is not duplicate-free canonical JSON",
        ));
    }
    Ok(head)
}

fn map_open_store_error(error: WorkingStoreError) -> Failure {
    match error {
        WorkingStoreError::MissingRoot(_) => Failure::new(
            "AUTHORING_REVISION3_EXPORT_ROOT_UNAVAILABLE",
            error.to_string(),
        ),
        WorkingStoreError::UnsafePath { .. } | WorkingStoreError::Io(_) => Failure::new(
            "AUTHORING_REVISION3_EXPORT_STORE_CHANGED",
            error.to_string(),
        ),
        WorkingStoreError::LimitExceeded { .. } => {
            Failure::new("AUTHORING_REVISION3_EXPORT_INPUT_LIMIT", error.to_string())
        }
        _ => invariant_failure(),
    }
}

fn map_export_error(error: Revision3ExactSnapshotExportErrorV1) -> Failure {
    let code = export_error_code(&error);
    Failure::new(code, error.to_string())
}

fn export_error_code(error: &Revision3ExactSnapshotExportErrorV1) -> &'static str {
    match error {
        Revision3ExactSnapshotExportErrorV1::Store(store_error) => match store_error {
            WorkingStoreError::HeadConflict { .. } | WorkingStoreError::MissingHead(_) => {
                "AUTHORING_REVISION3_EXPORT_HEAD_CONFLICT"
            }
            WorkingStoreError::MissingRoot(_)
            | WorkingStoreError::UnsafePath { .. }
            | WorkingStoreError::Io(_) => "AUTHORING_REVISION3_EXPORT_STORE_CHANGED",
            WorkingStoreError::MissingObject(path)
                if path.file_name().and_then(|name| name.to_str()) == Some("gore-project.json") =>
            {
                "AUTHORING_REVISION3_EXPORT_HEAD_CONFLICT"
            }
            WorkingStoreError::LimitExceeded { .. } => "AUTHORING_REVISION3_EXPORT_CLOSURE_LIMIT",
            WorkingStoreError::MissingObject(_)
            | WorkingStoreError::SealMismatch { .. }
            | WorkingStoreError::Collision { .. }
            | WorkingStoreError::InvalidJson { .. }
            | WorkingStoreError::NonCanonicalJson { .. }
            | WorkingStoreError::InvalidOgg(_)
            | WorkingStoreError::OggMetadataMismatch { .. } => {
                "AUTHORING_REVISION3_EXPORT_CLOSURE_INVALID"
            }
            WorkingStoreError::StagingCleanup { .. } => "AUTHORING_REVISION3_EXPORT_CLEANUP_FAILED",
            WorkingStoreError::InvalidLimits(_) | WorkingStoreError::Invariant(_) => {
                "AUTHORING_REVISION3_EXPORT_INVARIANT"
            }
        },
        Revision3ExactSnapshotExportErrorV1::InvalidOutput(_) => {
            "AUTHORING_REVISION3_EXPORT_OUTPUT_INVALID"
        }
        Revision3ExactSnapshotExportErrorV1::OutputAlreadyExists => {
            "AUTHORING_REVISION3_EXPORT_OUTPUT_EXISTS"
        }
        Revision3ExactSnapshotExportErrorV1::InvalidClosure(_) => {
            "AUTHORING_REVISION3_EXPORT_CLOSURE_INVALID"
        }
        Revision3ExactSnapshotExportErrorV1::ClosureLimit { .. } => {
            "AUTHORING_REVISION3_EXPORT_CLOSURE_LIMIT"
        }
        Revision3ExactSnapshotExportErrorV1::Archive(_) => {
            "AUTHORING_REVISION3_EXPORT_ARCHIVE_FAILED"
        }
        Revision3ExactSnapshotExportErrorV1::Verification(_) => {
            "AUTHORING_REVISION3_EXPORT_VERIFY_FAILED"
        }
        Revision3ExactSnapshotExportErrorV1::Publication(_) => {
            "AUTHORING_REVISION3_EXPORT_PUBLICATION_FAILED"
        }
        Revision3ExactSnapshotExportErrorV1::StagingCleanup { primary, .. } => {
            export_error_code(primary)
        }
    }
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_EXPORT_REQUEST_INVALID",
        "managed snapshot export request must be one exact duplicate-free object",
    )
}

fn invariant_failure() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_EXPORT_INVARIANT",
        "managed snapshot export response invariant failed",
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
    use std::fs;
    use std::io::{self, Read};
    use std::path::PathBuf;

    use gore_authoring::{
        ContentSeal, ProjectId, ProjectRevision3, Revision3ExactSnapshotClosureV1,
        Revision3ExactSnapshotClosureV2, Revision3ExactSnapshotExportV1,
        Revision3ExactSnapshotExportV2, Sha256Digest, WorkingProjectStore, WorkingStoreLimits,
    };
    use serde_json::json;
    use sha2::{Digest, Sha256};
    use tempfile::TempDir;
    use zip::ZipArchive;

    use super::*;

    fn published_empty_store() -> (TempDir, String, String) {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("managed.goreproj");
        let store = WorkingProjectStore::at(&root, WorkingStoreLimits::default()).unwrap();
        let project: ProjectRevision3 = serde_json::from_value(json!({
            "format": 2,
            "schema_revision": 3,
            "project_id": "03030303030303030303030303030303",
            "revision": 7,
            "meta": {"name": "Exact export", "version": "1.2.0", "author": "tests"},
            "target": {"executable": {
                "byte_len": 123,
                "sha256": "4545454545454545454545454545454545454545454545454545454545454545"
            }},
            "authoring_locales": [],
            "entities": {},
            "asset_store": {"assets": {}}
        }))
        .unwrap();
        let prepared = store.prepare_revision3_checkpoint(None, &project).unwrap();
        fs::write(root.join("gore-project.json"), &prepared.head_bytes).unwrap();
        (
            temp,
            root.to_string_lossy().into_owned(),
            serde_json::to_string(&prepared.head).unwrap(),
        )
    }

    fn request(root: &str, head_json: &str, output: &str) -> String {
        json!({
            "command": COMMAND,
            "payload": {
                "expected_head_json": head_json,
                "output": output,
                "root": root,
            }
        })
        .to_string()
    }

    fn request_v2(root: &str, head_json: &str, output: &str) -> String {
        json!({
            "command": COMMAND_V2,
            "payload": {
                "expected_head_json": head_json,
                "output": output,
                "root": root,
            }
        })
        .to_string()
    }

    #[test]
    fn v2_wire_is_closed_version_specific_and_uses_the_same_caps() {
        let wrong_version = export_revision3_exact_snapshot_v2_raw(
            r#"{"command":"authoring_store_export_revision3_exact_snapshot_v1","payload":{"expected_head_json":"{}","output":"x","root":"x"}}"#,
        );
        assert_eq!(
            wrong_version["error"]["code"],
            "AUTHORING_REVISION3_EXPORT_REQUEST_INVALID"
        );

        let unknown = export_revision3_exact_snapshot_v2_raw(
            r#"{"command":"authoring_store_export_revision3_exact_snapshot_v2","payload":{"expected_head_json":"{}","output":"x","root":"x","future":true}}"#,
        );
        assert_eq!(
            unknown["error"]["code"],
            "AUTHORING_REVISION3_EXPORT_REQUEST_INVALID"
        );

        let duplicate = export_revision3_exact_snapshot_v2_raw(
            r#"{"command":"authoring_store_export_revision3_exact_snapshot_v2","command":"authoring_store_export_revision3_exact_snapshot_v2","payload":{"expected_head_json":"{}","output":"x","root":"x"}}"#,
        );
        assert_eq!(
            duplicate["error"]["code"],
            "AUTHORING_REVISION3_EXPORT_REQUEST_INVALID"
        );

        let oversized = export_revision3_exact_snapshot_v2_raw(&" ".repeat(MAX_WIRE_BYTES + 1));
        assert_eq!(
            oversized["error"]["code"],
            "AUTHORING_REVISION3_EXPORT_INPUT_LIMIT"
        );
        let oversized_path = "x".repeat(MAX_PATH_BYTES + 1);
        let path_response = export_revision3_exact_snapshot_v2_raw(
            &json!({
                "command": COMMAND_V2,
                "payload": {
                    "expected_head_json": "{}",
                    "output": oversized_path,
                    "root": "x",
                }
            })
            .to_string(),
        );
        assert_eq!(
            path_response["error"]["code"],
            "AUTHORING_REVISION3_EXPORT_REQUEST_INVALID"
        );
    }

    #[test]
    fn v2_response_receipt_and_v1_delta_are_exact() {
        let (temp, root, head_json) = published_empty_store();
        let v1_output = temp.path().join("exact-v1.goremod");
        let v2_output = temp.path().join("exact-v2.goremod");
        let v1_output_wire = v1_output.to_string_lossy().into_owned();
        let v2_output_wire = v2_output.to_string_lossy().into_owned();
        let v1 =
            export_revision3_exact_snapshot_v1_raw(&request(&root, &head_json, &v1_output_wire));
        let v2 =
            export_revision3_exact_snapshot_v2_raw(&request_v2(&root, &head_json, &v2_output_wire));

        assert_eq!(v1["ok"], true);
        assert_eq!(v2["ok"], true);
        assert_eq!(v1["format"], REVISION3_EXACT_SNAPSHOT_EXPORT_FORMAT_V1);
        assert_eq!(
            v1["artifact_kind"],
            REVISION3_EXACT_SNAPSHOT_ARTIFACT_KIND_V1
        );
        assert_eq!(
            v1["restore_status"],
            REVISION3_EXACT_SNAPSHOT_RESTORE_STATUS_V1
        );
        assert_eq!(v2["format"], REVISION3_EXACT_SNAPSHOT_EXPORT_FORMAT_V2);
        assert_eq!(
            v2["artifact_kind"],
            REVISION3_EXACT_SNAPSHOT_ARTIFACT_KIND_V2
        );
        assert_eq!(
            v2["restore_status"],
            REVISION3_EXACT_SNAPSHOT_RESTORE_STATUS_V2
        );
        assert_eq!(v2["basis_head_json"], head_json);
        assert_eq!(v2["project_id"], "03030303030303030303030303030303");
        assert_eq!(v2["project_revision"], 7);
        assert_eq!(v2["output"], v2_output_wire);
        assert_eq!(v2["outcome"], "exported");
        assert_eq!(v2["publication_status"], "published");
        assert_eq!(v2["retry_safe"], false);
        assert_eq!(v2["warning"], Value::Null);
        assert_eq!(v2["manifest"]["relative_name"], "gore-export.json");
        assert_eq!(v2["closure"]["snapshot_objects"], 1);
        assert_eq!(v2["closure"]["entity_objects"], 0);
        assert_eq!(v2["closure"]["asset_objects"], 0);
        assert_eq!(v2["closure"]["archive_entries"], 4);
        assert_eq!(v2.as_object().unwrap().len(), 21);

        let archive_bytes = fs::read(&v2_output).unwrap();
        let archive_seal: ContentSeal = serde_json::from_value(v2["archive"].clone()).unwrap();
        assert_eq!(archive_seal.byte_len, archive_bytes.len() as u64);
        assert_eq!(
            archive_seal.sha256,
            Sha256Digest::from_bytes(Sha256::digest(&archive_bytes).into())
        );
        let mut archive = ZipArchive::new(std::io::Cursor::new(&archive_bytes)).unwrap();
        assert_eq!(archive.len(), 4);
        let mut manifest_bytes = Vec::new();
        archive
            .by_name(REVISION3_EXACT_SNAPSHOT_MANIFEST_FILE_V2)
            .unwrap()
            .read_to_end(&mut manifest_bytes)
            .unwrap();
        let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
        assert_eq!(manifest["format"], "gore.managed-project-snapshot.v2");
        assert_eq!(manifest["schema"], 2);
        assert_eq!(
            manifest["artifact_kind"],
            "portable_snapshot_restorable_copy"
        );
        assert_eq!(manifest["restore_status"], "supported");
        assert_eq!(manifest["members"].as_array().unwrap().len(), 3);
        assert_eq!(v2["manifest"]["byte_len"], manifest_bytes.len() as u64);
        assert_eq!(
            v2["manifest"]["sha256"],
            Sha256Digest::from_bytes(Sha256::digest(&manifest_bytes).into()).to_string()
        );

        let mut v1_common = v1.as_object().unwrap().clone();
        let mut v2_common = v2.as_object().unwrap().clone();
        for key in [
            "format",
            "artifact_kind",
            "restore_status",
            "output",
            "archive",
            "manifest",
            "closure",
        ] {
            v1_common.remove(key);
            v2_common.remove(key);
        }
        assert_eq!(v1_common, v2_common);
        let mut v1_closure = v1["closure"].as_object().unwrap().clone();
        let mut v2_closure = v2["closure"].as_object().unwrap().clone();
        v1_closure.remove("uncompressed_bytes");
        v2_closure.remove("uncompressed_bytes");
        assert_eq!(v1_closure, v2_closure);
    }

    #[test]
    fn exact_wire_rejects_unknown_duplicate_and_noncanonical_head_fields() {
        let invalid = export_revision3_exact_snapshot_v1_raw(
            r#"{"command":"authoring_store_export_revision3_exact_snapshot_v1","payload":{"expected_head_json":"{}","output":"x","root":"x","future":true}}"#,
        );
        assert_eq!(
            invalid["error"]["code"],
            "AUTHORING_REVISION3_EXPORT_REQUEST_INVALID"
        );

        let duplicate = export_revision3_exact_snapshot_v1_raw(
            r#"{"command":"authoring_store_export_revision3_exact_snapshot_v1","command":"authoring_store_export_revision3_exact_snapshot_v1","payload":{"expected_head_json":"{}","output":"x","root":"x"}}"#,
        );
        assert_eq!(
            duplicate["error"]["code"],
            "AUTHORING_REVISION3_EXPORT_REQUEST_INVALID"
        );

        let noncanonical_head = export_revision3_exact_snapshot_v1_raw(
            &json!({
                "command": COMMAND,
                "payload": {
                    "expected_head_json": "{ \"store_format\":1,\"snapshot\":{\"byte_len\":1,\"sha256\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}}",
                    "output": "x",
                    "root": "x",
                }
            })
            .to_string(),
        );
        assert_eq!(
            noncanonical_head["error"]["code"],
            "AUTHORING_REVISION3_EXPORT_HEAD_INVALID"
        );

        let duplicate_payload_field = export_revision3_exact_snapshot_v1_raw(
            r#"{"command":"authoring_store_export_revision3_exact_snapshot_v1","payload":{"expected_head_json":"{}","expected_head_json":"{}","output":"x","root":"x"}}"#,
        );
        assert_eq!(
            duplicate_payload_field["error"]["code"],
            "AUTHORING_REVISION3_EXPORT_REQUEST_INVALID"
        );

        let duplicate_head_field = export_revision3_exact_snapshot_v1_raw(
            &json!({
                "command": COMMAND,
                "payload": {
                    "expected_head_json": "{\"store_format\":1,\"store_format\":1,\"snapshot\":{\"byte_len\":1,\"sha256\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}}",
                    "output": "x",
                    "root": "x",
                }
            })
            .to_string(),
        );
        assert_eq!(
            duplicate_head_field["error"]["code"],
            "AUTHORING_REVISION3_EXPORT_HEAD_INVALID"
        );
    }

    #[test]
    fn exact_wire_enforces_each_transport_limit_before_store_access() {
        let oversized_wire = " ".repeat(MAX_WIRE_BYTES + 1);
        let wire_response = export_revision3_exact_snapshot_v1_raw(&oversized_wire);
        assert_eq!(
            wire_response["error"]["code"],
            "AUTHORING_REVISION3_EXPORT_INPUT_LIMIT"
        );

        let oversized_path = "x".repeat(MAX_PATH_BYTES + 1);
        let path_response = export_revision3_exact_snapshot_v1_raw(
            &json!({
                "command": COMMAND,
                "payload": {
                    "expected_head_json": "{}",
                    "output": oversized_path,
                    "root": "x",
                }
            })
            .to_string(),
        );
        assert_eq!(
            path_response["error"]["code"],
            "AUTHORING_REVISION3_EXPORT_REQUEST_INVALID"
        );

        let oversized_head = "x".repeat(MAX_HEAD_JSON_BYTES + 1);
        let head_response = export_revision3_exact_snapshot_v1_raw(
            &json!({
                "command": COMMAND,
                "payload": {
                    "expected_head_json": oversized_head,
                    "output": "x",
                    "root": "x",
                }
            })
            .to_string(),
        );
        assert_eq!(
            head_response["error"]["code"],
            "AUTHORING_REVISION3_EXPORT_HEAD_INVALID"
        );
    }

    #[test]
    fn export_response_is_closed_and_preserves_exact_output_spelling() {
        let (temp, root, head_json) = published_empty_store();
        let output = temp.path().join("Review Copy.goremod");
        let output_wire = output.to_string_lossy().into_owned();
        let response =
            export_revision3_exact_snapshot_v1_raw(&request(&root, &head_json, &output_wire));

        assert_eq!(response["ok"], true);
        assert_eq!(response["outcome"], "exported");
        assert_eq!(
            response["format"],
            REVISION3_EXACT_SNAPSHOT_EXPORT_FORMAT_V1
        );
        assert_eq!(response["output"], output_wire);
        assert_eq!(response["basis_head_json"], head_json);
        assert_eq!(response["project_revision"], 7);
        assert_eq!(response["closure"]["snapshot_objects"], 1);
        assert_eq!(response["closure"]["entity_objects"], 0);
        assert_eq!(response["closure"]["asset_objects"], 0);
        assert_eq!(response["closure"]["archive_entries"], 4);
        assert_eq!(response["publication_status"], "published");
        assert_eq!(response["retry_safe"], false);
        assert_eq!(response["warning"], Value::Null);
        assert!(output.is_file());

        let keys = response
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(keys.len(), 21);
        for required in [
            "ok",
            "outcome",
            "format",
            "artifact_kind",
            "restore_status",
            "basis_head_json",
            "project_id",
            "project_revision",
            "output",
            "archive",
            "manifest",
            "closure",
            "publication_status",
            "retry_safe",
            "warning",
            "project_mutation",
            "game_mutation",
            "save_mutation",
            "build_status",
            "deployment_status",
            "runtime_status",
        ] {
            assert!(response.get(required).is_some(), "missing {required}");
        }
    }

    #[test]
    fn public_dispatch_routes_both_closed_exports_without_extra_authority() {
        let (temp, root, head_json) = published_empty_store();
        let output = temp.path().join("public-dispatch.goremod");
        let response: Value = serde_json::from_str(&crate::execute_json(&request(
            &root,
            &head_json,
            &output.to_string_lossy(),
        )))
        .unwrap();
        assert_eq!(response["ok"], true);
        assert_eq!(response["outcome"], "exported");
        assert!(output.is_file());

        let v2_output = temp.path().join("public-dispatch-v2.goremod");
        let v2_response: Value = serde_json::from_str(&crate::execute_json(&request_v2(
            &root,
            &head_json,
            &v2_output.to_string_lossy(),
        )))
        .unwrap();
        assert_eq!(v2_response["ok"], true);
        assert_eq!(v2_response["outcome"], "exported");
        assert_eq!(
            v2_response["format"],
            REVISION3_EXACT_SNAPSHOT_EXPORT_FORMAT_V2
        );
        assert_eq!(v2_response["restore_status"], "supported");
        assert!(v2_output.is_file());

        let forbidden: Value = serde_json::from_str(&crate::execute_json(
            &json!({
                "command": COMMAND,
                "payload": {
                    "expected_head_json": head_json,
                    "output": temp.path().join("forbidden.goremod").to_string_lossy(),
                    "root": root,
                    "game_root": "C:\\Games\\Gore",
                }
            })
            .to_string(),
        ))
        .unwrap();
        assert_eq!(
            forbidden["error"]["code"],
            "AUTHORING_REVISION3_EXPORT_REQUEST_INVALID"
        );
    }

    #[test]
    fn existing_output_is_never_clobbered() {
        let (temp, root, head_json) = published_empty_store();
        let output = temp.path().join("existing.goremod");
        fs::write(&output, b"keep me").unwrap();
        let output_wire = output.to_string_lossy().into_owned();

        let response =
            export_revision3_exact_snapshot_v1_raw(&request(&root, &head_json, &output_wire));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_EXPORT_OUTPUT_EXISTS"
        );
        assert_eq!(fs::read(output).unwrap(), b"keep me");
    }

    #[test]
    fn stale_head_bad_output_missing_root_and_store_corruption_have_exact_codes() {
        let (temp, root, head_json) = published_empty_store();
        let output = temp.path().join("codes.goremod");
        let output_wire = output.to_string_lossy().into_owned();

        let mut stale: WorkingHead = serde_json::from_str(&head_json).unwrap();
        stale.snapshot.sha256 = Sha256Digest::from_bytes([9; 32]);
        let stale_response = export_revision3_exact_snapshot_v1_raw(&request(
            &root,
            &serde_json::to_string(&stale).unwrap(),
            &output_wire,
        ));
        assert_eq!(
            stale_response["error"]["code"],
            "AUTHORING_REVISION3_EXPORT_HEAD_CONFLICT"
        );

        let bad_output = temp.path().join("codes.zip").to_string_lossy().into_owned();
        let output_response =
            export_revision3_exact_snapshot_v1_raw(&request(&root, &head_json, &bad_output));
        assert_eq!(
            output_response["error"]["code"],
            "AUTHORING_REVISION3_EXPORT_OUTPUT_INVALID"
        );

        let missing_root = temp.path().join("missing.goreproj");
        let root_response = export_revision3_exact_snapshot_v1_raw(&request(
            &missing_root.to_string_lossy(),
            &head_json,
            &output_wire,
        ));
        assert_eq!(
            root_response["error"]["code"],
            "AUTHORING_REVISION3_EXPORT_ROOT_UNAVAILABLE"
        );

        let head: WorkingHead = serde_json::from_str(&head_json).unwrap();
        let digest = head.snapshot.sha256.to_string();
        let snapshot = PathBuf::from(&root)
            .join("snapshots")
            .join("sha256")
            .join(&digest[..2])
            .join(format!("{}.json", &digest[2..]));
        let mut bytes = fs::read(&snapshot).unwrap();
        bytes[0] ^= 1;
        fs::write(snapshot, bytes).unwrap();
        let corrupt_response =
            export_revision3_exact_snapshot_v1_raw(&request(&root, &head_json, &output_wire));
        assert_eq!(
            corrupt_response["error"]["code"],
            "AUTHORING_REVISION3_EXPORT_CLOSURE_INVALID"
        );
        assert!(!output.exists());
    }

    #[test]
    fn store_integrity_errors_are_not_misclassified_as_retryable_output_failures() {
        let unsafe_failure = map_export_error(Revision3ExactSnapshotExportErrorV1::Store(
            WorkingStoreError::UnsafePath {
                path: PathBuf::from("store-object"),
                reason: "injected link".to_owned(),
            },
        ));
        assert_eq!(
            unsafe_failure.code,
            "AUTHORING_REVISION3_EXPORT_STORE_CHANGED"
        );
        let io_failure = map_export_error(Revision3ExactSnapshotExportErrorV1::Store(
            WorkingStoreError::Io(io::Error::new(io::ErrorKind::Other, "injected read")),
        ));
        assert_eq!(io_failure.code, "AUTHORING_REVISION3_EXPORT_STORE_CHANGED");
        assert_eq!(
            map_export_error(Revision3ExactSnapshotExportErrorV1::InvalidOutput(
                "injected output".to_owned()
            ))
            .code,
            "AUTHORING_REVISION3_EXPORT_OUTPUT_INVALID"
        );
        assert_eq!(
            map_export_error(Revision3ExactSnapshotExportErrorV1::Verification(
                "injected staged reopen".to_owned()
            ))
            .code,
            "AUTHORING_REVISION3_EXPORT_VERIFY_FAILED"
        );
        let cleanup_with_store_primary =
            map_export_error(Revision3ExactSnapshotExportErrorV1::StagingCleanup {
                primary: Box::new(Revision3ExactSnapshotExportErrorV1::Store(
                    WorkingStoreError::SealMismatch {
                        path: PathBuf::from("store-object"),
                        reason: "injected drift".to_owned(),
                    },
                )),
                cleanup: "injected cleanup failure".to_owned(),
            });
        assert_eq!(
            cleanup_with_store_primary.code,
            "AUTHORING_REVISION3_EXPORT_CLOSURE_INVALID"
        );
    }

    #[test]
    fn all_publication_terminals_have_the_exact_closed_warning_contract() {
        let head: WorkingHead = serde_json::from_str(
            r#"{"store_format":1,"snapshot":{"byte_len":123,"sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}"#,
        )
        .unwrap();
        let receipt = Revision3ExactSnapshotExportV1 {
            head,
            project_id: ProjectId::from_bytes([3; 16]),
            project_revision: 7,
            archive: ContentSeal {
                byte_len: 4096,
                sha256: Sha256Digest::from_bytes([12; 32]),
            },
            manifest: ContentSeal {
                byte_len: 512,
                sha256: Sha256Digest::from_bytes([13; 32]),
            },
            closure: Revision3ExactSnapshotClosureV1 {
                snapshot_objects: 1,
                entity_objects: 0,
                asset_objects: 0,
                archive_entries: 4,
                uncompressed_bytes: 1024,
            },
        };
        let basis = serde_json::to_string(&receipt.head).unwrap();

        let cases = [
            (
                Revision3ExactSnapshotExportPublicationV1::Exported(receipt.clone()),
                "exported",
                "published",
                None,
            ),
            (
                Revision3ExactSnapshotExportPublicationV1::ExportedWithCleanupWarning(
                    receipt.clone(),
                ),
                "exported_with_cleanup_warning",
                "published_with_cleanup_warning",
                Some(CLEANUP_WARNING_CODE),
            ),
            (
                Revision3ExactSnapshotExportPublicationV1::PublicationUncertain(receipt),
                "publication_uncertain",
                "publication_uncertain",
                Some(UNCERTAIN_WARNING_CODE),
            ),
        ];
        for (publication, outcome, status, warning_code) in cases {
            let response = publication_response(
                publication,
                &serde_json::from_str(&basis).unwrap(),
                basis.clone(),
                "C:\\Exports\\Review.goremod".to_owned(),
            );
            assert_eq!(response["ok"], true);
            assert_eq!(response["outcome"], outcome);
            assert_eq!(response["publication_status"], status);
            assert_eq!(response["retry_safe"], false);
            match warning_code {
                Some(code) => assert_eq!(response["warning"]["code"], code),
                None => assert_eq!(response["warning"], Value::Null),
            }
        }
    }

    #[test]
    fn all_v2_publication_terminals_have_the_exact_closed_warning_contract() {
        let head: WorkingHead = serde_json::from_str(
            r#"{"store_format":1,"snapshot":{"byte_len":123,"sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}"#,
        )
        .unwrap();
        let receipt = Revision3ExactSnapshotExportV2 {
            head,
            project_id: ProjectId::from_bytes([3; 16]),
            project_revision: 7,
            archive: ContentSeal {
                byte_len: 4096,
                sha256: Sha256Digest::from_bytes([12; 32]),
            },
            manifest: ContentSeal {
                byte_len: 512,
                sha256: Sha256Digest::from_bytes([13; 32]),
            },
            closure: Revision3ExactSnapshotClosureV2 {
                snapshot_objects: 1,
                entity_objects: 0,
                asset_objects: 0,
                archive_entries: 4,
                uncompressed_bytes: 1024,
            },
        };
        let basis = serde_json::to_string(&receipt.head).unwrap();

        let cases = [
            (
                Revision3ExactSnapshotExportPublicationV2::Exported(receipt.clone()),
                "exported",
                "published",
                None,
            ),
            (
                Revision3ExactSnapshotExportPublicationV2::ExportedWithCleanupWarning(
                    receipt.clone(),
                ),
                "exported_with_cleanup_warning",
                "published_with_cleanup_warning",
                Some(CLEANUP_WARNING_CODE),
            ),
            (
                Revision3ExactSnapshotExportPublicationV2::PublicationUncertain(receipt),
                "publication_uncertain",
                "publication_uncertain",
                Some(UNCERTAIN_WARNING_CODE),
            ),
        ];
        for (publication, outcome, status, warning_code) in cases {
            let response = publication_response_v2(
                publication,
                &serde_json::from_str(&basis).unwrap(),
                basis.clone(),
                "C:\\Exports\\Restorable.goremod".to_owned(),
            );
            assert_eq!(response["ok"], true);
            assert_eq!(response["outcome"], outcome);
            assert_eq!(response["publication_status"], status);
            assert_eq!(response["retry_safe"], false);
            assert_eq!(
                response["format"],
                REVISION3_EXACT_SNAPSHOT_EXPORT_FORMAT_V2
            );
            assert_eq!(
                response["artifact_kind"],
                REVISION3_EXACT_SNAPSHOT_ARTIFACT_KIND_V2
            );
            assert_eq!(
                response["restore_status"],
                REVISION3_EXACT_SNAPSHOT_RESTORE_STATUS_V2
            );
            match warning_code {
                Some(code) => assert_eq!(response["warning"]["code"], code),
                None => assert_eq!(response["warning"], Value::Null),
            }
        }
    }

    #[test]
    fn receipt_head_mismatch_can_only_be_publication_uncertain() {
        let expected_head: WorkingHead = serde_json::from_str(
            r#"{"store_format":1,"snapshot":{"byte_len":123,"sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}"#,
        )
        .unwrap();
        let receipt_head: WorkingHead = serde_json::from_str(
            r#"{"store_format":1,"snapshot":{"byte_len":123,"sha256":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"}}"#,
        )
        .unwrap();
        let receipt = Revision3ExactSnapshotExportV1 {
            head: receipt_head,
            project_id: ProjectId::from_bytes([3; 16]),
            project_revision: 7,
            archive: ContentSeal {
                byte_len: 4096,
                sha256: Sha256Digest::from_bytes([12; 32]),
            },
            manifest: ContentSeal {
                byte_len: 512,
                sha256: Sha256Digest::from_bytes([13; 32]),
            },
            closure: Revision3ExactSnapshotClosureV1 {
                snapshot_objects: 1,
                entity_objects: 0,
                asset_objects: 0,
                archive_entries: 4,
                uncompressed_bytes: 1024,
            },
        };
        let basis = serde_json::to_string(&expected_head).unwrap();
        let response = publication_response(
            Revision3ExactSnapshotExportPublicationV1::Exported(receipt),
            &expected_head,
            basis,
            "C:\\Exports\\Review.goremod".to_owned(),
        );

        assert_eq!(response["ok"], true);
        assert_eq!(response["outcome"], "publication_uncertain");
        assert_eq!(response["publication_status"], "publication_uncertain");
        assert_eq!(response["retry_safe"], false);
        assert_eq!(
            response["warning"]["code"],
            "AUTHORING_REVISION3_EXPORT_PUBLICATION_UNCERTAIN"
        );
    }
}
