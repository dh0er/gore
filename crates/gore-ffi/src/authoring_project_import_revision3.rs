//! Inspection and destination materialization for one untrusted restorable managed R3 snapshot.
//!
//! The inspection command accepts only a source path and remains strictly read-only. The separate
//! import command accepts that source, an absent destination, and the exact inspected archive CAS;
//! native authoring code verifies and streams through one retained source handle before atomically
//! publishing a new managed directory. Neither command can adopt a Studio session, read or mutate
//! a game/save, build, deploy, launch, or claim runtime qualification.

use std::path::Path;

use gore_authoring::{
    import_revision3_exact_snapshot_v2, inspect_revision3_exact_snapshot_v2, ContentSeal,
    Revision3ExactSnapshotImportErrorV2, Revision3ExactSnapshotImportPublicationV2,
    Revision3ExactSnapshotImportV2, Revision3ExactSnapshotInspectionErrorV2,
    Revision3ExactSnapshotInspectionV2, REVISION3_EXACT_SNAPSHOT_IMPORT_ARTIFACT_KIND_V2,
    REVISION3_EXACT_SNAPSHOT_IMPORT_FORMAT_V2, REVISION3_EXACT_SNAPSHOT_IMPORT_MANIFEST_FILE_V2,
    REVISION3_EXACT_SNAPSHOT_IMPORT_RESTORE_STATUS_V2,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_inspect_revision3_exact_snapshot_v2";
pub(super) const IMPORT_COMMAND: &str = "authoring_store_import_revision3_exact_snapshot_v2";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_WIRE_BYTES: usize = MAX_PATH_BYTES * 6 + 1024;
const MAX_IMPORT_WIRE_BYTES: usize = MAX_PATH_BYTES * 12 + 2048;
const MAX_MANIFEST_BYTES: u64 = 128 * 1024 * 1024;
const MAX_SNAPSHOT_OBJECTS: u64 = 100_000;
const MAX_ENTITY_OBJECTS: u64 = 100_000;
const MAX_ASSET_OBJECTS: u64 = 100_000;
const MAX_CLOSURE_OBJECTS: u64 = 300_000;
const MAX_ARCHIVE_ENTRIES: u64 = 300_003;
const MAX_ARCHIVE_BYTES: u64 = 70 * 1024 * 1024 * 1024;
const MAX_UNCOMPRESSED_BYTES: u64 = 70 * 1024 * 1024 * 1024;
const MAX_SIGNED_WIRE_INTEGER: u64 = i64::MAX as u64;

const REQUEST_INVALID_CODE: &str = "AUTHORING_REVISION3_IMPORT_REQUEST_INVALID";
const LIMIT_CODE: &str = "AUTHORING_REVISION3_IMPORT_LIMIT";
const SOURCE_INVALID_CODE: &str = "AUTHORING_REVISION3_IMPORT_SOURCE_INVALID";
const PLATFORM_UNSUPPORTED_CODE: &str = "AUTHORING_REVISION3_IMPORT_PLATFORM_UNSUPPORTED";
const UNSUPPORTED_REVIEW_COPY_CODE: &str = "AUTHORING_REVISION3_IMPORT_UNSUPPORTED_REVIEW_COPY";
const ARCHIVE_INVALID_CODE: &str = "AUTHORING_REVISION3_IMPORT_ARCHIVE_INVALID";
const MANIFEST_INVALID_CODE: &str = "AUTHORING_REVISION3_IMPORT_MANIFEST_INVALID";
const CLOSURE_INVALID_CODE: &str = "AUTHORING_REVISION3_IMPORT_CLOSURE_INVALID";
const INVARIANT_CODE: &str = "AUTHORING_REVISION3_IMPORT_INVARIANT";
const DESTINATION_INVALID_CODE: &str = "AUTHORING_REVISION3_IMPORT_DESTINATION_INVALID";
const SOURCE_CHANGED_CODE: &str = "AUTHORING_REVISION3_IMPORT_SOURCE_CHANGED";
const MATERIALIZATION_FAILED_CODE: &str = "AUTHORING_REVISION3_IMPORT_MATERIALIZATION_FAILED";
const VERIFICATION_FAILED_CODE: &str = "AUTHORING_REVISION3_IMPORT_VERIFICATION_FAILED";
const PUBLICATION_FAILED_CODE: &str = "AUTHORING_REVISION3_IMPORT_PUBLICATION_FAILED";
const CLEANUP_FAILED_CODE: &str = "AUTHORING_REVISION3_IMPORT_CLEANUP_FAILED";
const CLEANUP_WARNING_CODE: &str = "AUTHORING_REVISION3_IMPORT_CLEANUP_WARNING";
const CLEANUP_WARNING_MESSAGE: &str =
    "the verified project was materialized, but private staging cleanup was incomplete";
const PUBLICATION_UNCERTAIN_CODE: &str = "AUTHORING_REVISION3_IMPORT_PUBLICATION_UNCERTAIN";
const PUBLICATION_UNCERTAIN_MESSAGE: &str =
    "project publication may have completed; do not retry automatically";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest {
    command: String,
    payload: InspectSnapshotWirePayload,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectSnapshotWirePayload {
    source: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactImportWireRequest {
    command: String,
    payload: ImportSnapshotWirePayload,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportSnapshotWirePayload {
    source: String,
    destination: String,
    expected_archive: ContentSeal,
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

pub(super) fn inspect_revision3_exact_snapshot_v2_raw(input: &str) -> Value {
    inspect_revision3_exact_snapshot_v2_inner(input).unwrap_or_else(Failure::response)
}

pub(super) fn import_revision3_exact_snapshot_v2_raw(input: &str) -> Value {
    import_revision3_exact_snapshot_v2_inner(input).unwrap_or_else(Failure::response)
}

fn inspect_revision3_exact_snapshot_v2_inner(input: &str) -> Result<Value, Failure> {
    let payload = parse_exact_wire(input)?;
    let source = payload.source;
    let inspection =
        inspect_revision3_exact_snapshot_v2(Path::new(&source)).map_err(map_inspection_error)?;
    inspection_response(inspection, source)
}

fn import_revision3_exact_snapshot_v2_inner(input: &str) -> Result<Value, Failure> {
    let payload = parse_exact_import_wire(input)?;
    let source = payload.source;
    let destination = payload.destination;
    let expected_archive = payload.expected_archive;
    let publication = import_revision3_exact_snapshot_v2(
        Path::new(&source),
        &expected_archive,
        Path::new(&destination),
    )
    .map_err(map_import_error)?;
    Ok(import_publication_response(
        publication,
        source,
        destination,
        &expected_archive,
    ))
}

fn parse_exact_wire(input: &str) -> Result<InspectSnapshotWirePayload, Failure> {
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            LIMIT_CODE,
            "managed snapshot inspection request exceeds its closed wire limit",
        ));
    }
    let request: ExactWireRequest = serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    validate_source_spelling(&request.payload.source)?;
    Ok(request.payload)
}

fn parse_exact_import_wire(input: &str) -> Result<ImportSnapshotWirePayload, Failure> {
    if input.len() > MAX_IMPORT_WIRE_BYTES {
        return Err(Failure::new(
            LIMIT_CODE,
            "managed snapshot import request exceeds its closed wire limit",
        ));
    }
    let request: ExactImportWireRequest =
        serde_json::from_str(input).map_err(|_| invalid_import_request())?;
    if request.command != IMPORT_COMMAND {
        return Err(invalid_import_request());
    }
    validate_source_spelling(&request.payload.source)?;
    validate_destination_spelling(&request.payload.destination)?;
    if request.payload.source == request.payload.destination {
        return Err(Failure::new(
            DESTINATION_INVALID_CODE,
            "managed snapshot source and destination must be distinct",
        ));
    }
    if request.payload.expected_archive.byte_len == 0
        || request.payload.expected_archive.byte_len > MAX_ARCHIVE_BYTES
    {
        return Err(Failure::new(
            LIMIT_CODE,
            "expected managed snapshot archive seal exceeds its closed byte range",
        ));
    }
    Ok(request.payload)
}

fn validate_source_spelling(source: &str) -> Result<(), Failure> {
    if source.len() > MAX_PATH_BYTES {
        return Err(Failure::new(
            LIMIT_CODE,
            "managed snapshot source exceeds its closed path limit",
        ));
    }
    if source.is_empty() || source.contains('\0') {
        return Err(Failure::new(
            SOURCE_INVALID_CODE,
            "managed snapshot source is not one bounded file spelling",
        ));
    }
    Ok(())
}

fn validate_destination_spelling(destination: &str) -> Result<(), Failure> {
    if destination.len() > MAX_PATH_BYTES {
        return Err(Failure::new(
            LIMIT_CODE,
            "managed snapshot destination exceeds its closed path limit",
        ));
    }
    if destination.is_empty() || destination.contains('\0') {
        return Err(Failure::new(
            DESTINATION_INVALID_CODE,
            "managed snapshot destination is not one bounded directory spelling",
        ));
    }
    Ok(())
}

fn inspection_response(
    inspection: Revision3ExactSnapshotInspectionV2,
    source: String,
) -> Result<Value, Failure> {
    validate_inspection_receipt(&inspection)?;
    let head_json = serde_json::to_string(&inspection.head).map_err(|_| invariant_failure())?;
    let project_id = inspection.project_id.to_string();

    Ok(json!({
        "ok": true,
        "outcome": "inspected_restorable_copy",
        "source": source,
        "format": REVISION3_EXACT_SNAPSHOT_IMPORT_FORMAT_V2,
        "artifact_kind": REVISION3_EXACT_SNAPSHOT_IMPORT_ARTIFACT_KIND_V2,
        "restore_status": REVISION3_EXACT_SNAPSHOT_IMPORT_RESTORE_STATUS_V2,
        "archive": inspection.archive,
        "manifest": {
            "relative_name": REVISION3_EXACT_SNAPSHOT_IMPORT_MANIFEST_FILE_V2,
            "byte_len": inspection.manifest.byte_len,
            "sha256": inspection.manifest.sha256,
        },
        "project_id": project_id,
        "project_revision": inspection.project_revision,
        "head_json": head_json,
        "closure": inspection.closure,
        "inspection_status": "verified_exact",
        "import_status": "not_performed",
        "project_mutation": "not_performed",
        "game_mutation": "not_performed",
        "save_mutation": "not_performed",
        "build_status": "not_performed",
        "deployment_status": "not_performed",
        "runtime_status": "runtime_unqualified",
        "publication_status": "not_supported",
        "retry_safe": true,
    }))
}

fn import_publication_response(
    publication: Revision3ExactSnapshotImportPublicationV2,
    source: String,
    destination: String,
    expected_archive: &ContentSeal,
) -> Value {
    match publication {
        Revision3ExactSnapshotImportPublicationV2::Imported(receipt) => confirmed_import_response(
            receipt,
            source,
            destination,
            expected_archive,
            "imported",
            "published",
            Value::Null,
        ),
        Revision3ExactSnapshotImportPublicationV2::ImportedWithCleanupWarning(receipt) => {
            confirmed_import_response(
                receipt,
                source,
                destination,
                expected_archive,
                "imported_with_cleanup_warning",
                "published_with_cleanup_warning",
                json!({
                    "code": CLEANUP_WARNING_CODE,
                    "message": CLEANUP_WARNING_MESSAGE,
                }),
            )
        }
        Revision3ExactSnapshotImportPublicationV2::PublicationUncertain => {
            uncertain_import_response(source, destination)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn confirmed_import_response(
    receipt: Revision3ExactSnapshotImportV2,
    source: String,
    destination: String,
    expected_archive: &ContentSeal,
    outcome: &'static str,
    publication_status: &'static str,
    warning: Value,
) -> Value {
    // The final directory is already visible when native returns a confirmed publication. Any
    // impossible bridge invariant must therefore become the same non-retryable uncertain terminal,
    // never an ordinary error that a caller might retry.
    if receipt.archive != *expected_archive || !valid_import_receipt(&receipt) {
        return uncertain_import_response(source, destination);
    }
    let Ok(head_json) = serde_json::to_string(&receipt.head) else {
        return uncertain_import_response(source, destination);
    };

    json!({
        "ok": true,
        "outcome": outcome,
        "source": source,
        "destination": destination,
        "format": REVISION3_EXACT_SNAPSHOT_IMPORT_FORMAT_V2,
        "artifact_kind": REVISION3_EXACT_SNAPSHOT_IMPORT_ARTIFACT_KIND_V2,
        "restore_status": REVISION3_EXACT_SNAPSHOT_IMPORT_RESTORE_STATUS_V2,
        "archive": receipt.archive,
        "manifest": {
            "relative_name": REVISION3_EXACT_SNAPSHOT_IMPORT_MANIFEST_FILE_V2,
            "byte_len": receipt.manifest.byte_len,
            "sha256": receipt.manifest.sha256,
        },
        "project_id": receipt.project_id.to_string(),
        "project_revision": receipt.project_revision,
        "head_json": head_json,
        "closure": receipt.closure,
        "inspection_status": "verified_exact",
        "import_status": "materialized",
        "project_mutation": "materialized",
        "session_adoption": "not_performed",
        "game_mutation": "not_performed",
        "save_mutation": "not_performed",
        "build_status": "not_performed",
        "deployment_status": "not_performed",
        "runtime_status": "runtime_unqualified",
        "publication_status": publication_status,
        "retry_safe": false,
        "warning": warning,
    })
}

fn uncertain_import_response(source: String, destination: String) -> Value {
    // Deliberately no archive/head/project/closure fields: publication uncertainty carries no
    // adoptable receipt at either the native or wire boundary.
    json!({
        "ok": true,
        "outcome": "publication_uncertain",
        "source": source,
        "destination": destination,
        "format": REVISION3_EXACT_SNAPSHOT_IMPORT_FORMAT_V2,
        "artifact_kind": REVISION3_EXACT_SNAPSHOT_IMPORT_ARTIFACT_KIND_V2,
        "restore_status": REVISION3_EXACT_SNAPSHOT_IMPORT_RESTORE_STATUS_V2,
        "inspection_status": "verified_exact",
        "import_status": "materialized",
        "project_mutation": "materialized",
        "session_adoption": "not_performed",
        "game_mutation": "not_performed",
        "save_mutation": "not_performed",
        "build_status": "not_performed",
        "deployment_status": "not_performed",
        "runtime_status": "runtime_unqualified",
        "publication_status": "publication_uncertain",
        "retry_safe": false,
        "warning": {
            "code": PUBLICATION_UNCERTAIN_CODE,
            "message": PUBLICATION_UNCERTAIN_MESSAGE,
        },
    })
}

fn valid_import_receipt(receipt: &Revision3ExactSnapshotImportV2) -> bool {
    validate_inspection_receipt(&Revision3ExactSnapshotInspectionV2 {
        head: receipt.head.clone(),
        project_id: receipt.project_id.clone(),
        project_revision: receipt.project_revision,
        archive: receipt.archive.clone(),
        manifest: receipt.manifest.clone(),
        closure: receipt.closure.clone(),
    })
    .is_ok()
}

fn validate_inspection_receipt(
    inspection: &Revision3ExactSnapshotInspectionV2,
) -> Result<(), Failure> {
    let closure = &inspection.closure;
    let store_objects = closure
        .snapshot_objects
        .checked_add(closure.entity_objects)
        .and_then(|count| count.checked_add(closure.asset_objects))
        .ok_or_else(invariant_failure)?;
    let expected_entries = store_objects.checked_add(3).ok_or_else(invariant_failure)?;
    let project_id = inspection.project_id.to_string();
    let canonical_id = project_id.len() == 32
        && project_id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && project_id != "00000000000000000000000000000000";

    if !canonical_id
        || inspection.project_revision > MAX_SIGNED_WIRE_INTEGER
        || inspection.archive.byte_len == 0
        || inspection.archive.byte_len > MAX_ARCHIVE_BYTES
        || inspection.archive.byte_len <= closure.uncompressed_bytes
        || inspection.manifest.byte_len == 0
        || inspection.manifest.byte_len > MAX_MANIFEST_BYTES
        || closure.snapshot_objects == 0
        || closure.snapshot_objects > MAX_SNAPSHOT_OBJECTS
        || closure.entity_objects > MAX_ENTITY_OBJECTS
        || closure.asset_objects > MAX_ASSET_OBJECTS
        || store_objects > MAX_CLOSURE_OBJECTS
        || closure.archive_entries != expected_entries
        || closure.archive_entries > MAX_ARCHIVE_ENTRIES
        || closure.uncompressed_bytes == 0
        || closure.uncompressed_bytes > MAX_UNCOMPRESSED_BYTES
        || inspection.manifest.byte_len > closure.uncompressed_bytes
        || inspection.head.snapshot.byte_len
            > closure
                .uncompressed_bytes
                .saturating_sub(inspection.manifest.byte_len)
    {
        return Err(invariant_failure());
    }
    Ok(())
}

fn map_inspection_error(error: Revision3ExactSnapshotInspectionErrorV2) -> Failure {
    match error {
        Revision3ExactSnapshotInspectionErrorV2::UnsupportedPlatform => Failure::new(
            PLATFORM_UNSUPPORTED_CODE,
            "managed snapshot inspection is not supported safely on this platform",
        ),
        Revision3ExactSnapshotInspectionErrorV2::UnsupportedReviewCopyV1 => Failure::new(
            UNSUPPORTED_REVIEW_COPY_CODE,
            "the selected snapshot declares the V1 review-copy format and is not restorable",
        ),
        Revision3ExactSnapshotInspectionErrorV2::InvalidSource(_) => Failure::new(
            SOURCE_INVALID_CODE,
            "the selected snapshot source could not be inspected as one safe regular file",
        ),
        Revision3ExactSnapshotInspectionErrorV2::Limit { .. } => Failure::new(
            LIMIT_CODE,
            "managed snapshot inspection exceeded a closed safety limit",
        ),
        Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(_) => Failure::new(
            ARCHIVE_INVALID_CODE,
            "the selected file is not one exact supported snapshot archive",
        ),
        Revision3ExactSnapshotInspectionErrorV2::InvalidManifest(_) => Failure::new(
            MANIFEST_INVALID_CODE,
            "the selected snapshot manifest is invalid or unsupported",
        ),
        Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(_) => Failure::new(
            CLOSURE_INVALID_CODE,
            "the selected snapshot does not contain one exact valid Store closure",
        ),
    }
}

fn map_import_error(error: Revision3ExactSnapshotImportErrorV2) -> Failure {
    match error {
        Revision3ExactSnapshotImportErrorV2::Inspection(
            Revision3ExactSnapshotInspectionErrorV2::UnsupportedPlatform,
        ) => map_inspection_error(Revision3ExactSnapshotInspectionErrorV2::UnsupportedPlatform),
        Revision3ExactSnapshotImportErrorV2::Inspection(_) => Failure::new(
            SOURCE_CHANGED_CODE,
            "the managed snapshot source no longer verifies as the inspected V2 archive",
        ),
        Revision3ExactSnapshotImportErrorV2::ArchiveCasMismatch { .. } => Failure::new(
            SOURCE_CHANGED_CODE,
            "the managed snapshot archive no longer matches the inspected archive seal",
        ),
        Revision3ExactSnapshotImportErrorV2::InvalidDestination(_)
        | Revision3ExactSnapshotImportErrorV2::DestinationAlreadyExists => Failure::new(
            DESTINATION_INVALID_CODE,
            "the managed snapshot destination is unavailable or no longer absent",
        ),
        Revision3ExactSnapshotImportErrorV2::Materialization(_) => Failure::new(
            MATERIALIZATION_FAILED_CODE,
            "the managed snapshot could not be materialized into private staging",
        ),
        Revision3ExactSnapshotImportErrorV2::CandidateVerification(_) => Failure::new(
            VERIFICATION_FAILED_CODE,
            "the materialized managed snapshot candidate failed exact verification",
        ),
        Revision3ExactSnapshotImportErrorV2::Publication(_) => Failure::new(
            PUBLICATION_FAILED_CODE,
            "the managed snapshot destination could not be published safely",
        ),
        Revision3ExactSnapshotImportErrorV2::StagingCleanup { .. } => Failure::new(
            CLEANUP_FAILED_CODE,
            "managed snapshot import failed and bounded private staging cleanup was incomplete",
        ),
    }
}

fn invalid_request() -> Failure {
    Failure::new(
        REQUEST_INVALID_CODE,
        "managed snapshot inspection request must contain only one exact command and source payload",
    )
}

fn invalid_import_request() -> Failure {
    Failure::new(
        REQUEST_INVALID_CODE,
        "managed snapshot import request must contain only one exact command, source, destination, and expected archive payload",
    )
}

fn invariant_failure() -> Failure {
    Failure::new(
        INVARIANT_CODE,
        "managed snapshot inspection response invariant failed",
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
    use std::collections::BTreeSet;
    #[cfg(windows)]
    use std::fs;
    #[cfg(windows)]
    use std::path::{Path, PathBuf};

    use gore_authoring::Revision3ExactSnapshotInspectionErrorV2;
    #[cfg(windows)]
    use gore_authoring::{
        AssetVerification, ProjectRevision3, Revision3ExactSnapshotExportPublicationV1,
        Revision3ExactSnapshotExportPublicationV2, Revision3ExactSnapshotInspectionV2, WorkingHead,
        WorkingProjectStore, WorkingStoreLimits,
    };
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;

    #[cfg(windows)]
    struct ExportedV2Fixture {
        _temp: TempDir,
        source: PathBuf,
        receipt: Revision3ExactSnapshotInspectionV2,
    }

    #[cfg(windows)]
    fn published_store() -> (TempDir, WorkingProjectStore, WorkingHead) {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("managed.goreproj");
        let store = WorkingProjectStore::at(&root, WorkingStoreLimits::default()).unwrap();
        let project: ProjectRevision3 = serde_json::from_value(json!({
            "format": 2,
            "schema_revision": 3,
            "project_id": "03030303030303030303030303030303",
            "revision": 7,
            "meta": {"name": "Exact import", "version": "1.2.0", "author": "tests"},
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
        (temp, store, prepared.head)
    }

    #[cfg(windows)]
    fn exported_v2_fixture() -> ExportedV2Fixture {
        let (temp, store, head) = published_store();
        let source = temp.path().join("Restorable Copy.goremod");
        let publication = store
            .export_current_revision3_exact_snapshot_v2(&head, &source)
            .unwrap();
        assert!(matches!(
            publication,
            Revision3ExactSnapshotExportPublicationV2::Exported(_)
        ));
        let receipt = inspect_revision3_exact_snapshot_v2(&source).unwrap();
        ExportedV2Fixture {
            _temp: temp,
            source,
            receipt,
        }
    }

    fn request(source: &str) -> String {
        json!({
            "command": COMMAND,
            "payload": {"source": source},
        })
        .to_string()
    }

    fn import_request(source: &str, destination: &str, expected_archive: &ContentSeal) -> String {
        json!({
            "command": IMPORT_COMMAND,
            "payload": {
                "source": source,
                "destination": destination,
                "expected_archive": expected_archive,
            },
        })
        .to_string()
    }

    fn exact_keys(value: &Value) -> BTreeSet<&str> {
        value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect()
    }

    #[cfg(windows)]
    fn directory_tree(root: &Path) -> Vec<PathBuf> {
        fn visit(root: &Path, path: &Path, entries: &mut Vec<PathBuf>) {
            let mut children = fs::read_dir(path)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect::<Vec<_>>();
            children.sort();
            for child in children {
                entries.push(child.strip_prefix(root).unwrap().to_owned());
                if child.is_dir() {
                    visit(root, &child, entries);
                }
            }
        }
        let mut entries = Vec::new();
        visit(root, root, &mut entries);
        entries
    }

    #[test]
    fn exact_wire_rejects_unknown_duplicate_missing_wrong_type_and_command() {
        let cases = [
            r#"{"command":"authoring_store_inspect_revision3_exact_snapshot_v2","payload":{"source":"x","future":true}}"#,
            r#"{"command":"authoring_store_inspect_revision3_exact_snapshot_v2","command":"authoring_store_inspect_revision3_exact_snapshot_v2","payload":{"source":"x"}}"#,
            r#"{"command":"authoring_store_inspect_revision3_exact_snapshot_v2","payload":{"source":"x","source":"y"}}"#,
            r#"{"command":"authoring_store_inspect_revision3_exact_snapshot_v2","payload":{}}"#,
            r#"{"command":"authoring_store_inspect_revision3_exact_snapshot_v2","payload":{"source":7}}"#,
            r#"{"command":"authoring_store_export_revision3_exact_snapshot_v2","payload":{"source":"x"}}"#,
            r#"{"command":"authoring_store_inspect_revision3_exact_snapshot_v2","payload":{"source":"x"},"root":"forbidden"}"#,
        ];
        for wire in cases {
            let response = inspect_revision3_exact_snapshot_v2_raw(wire);
            assert_eq!(response["error"]["code"], REQUEST_INVALID_CODE, "{wire}");
        }
    }

    #[test]
    fn wire_and_source_caps_fail_before_native_file_access() {
        let oversized_wire = " ".repeat(MAX_WIRE_BYTES + 1);
        assert_eq!(
            inspect_revision3_exact_snapshot_v2_raw(&oversized_wire)["error"]["code"],
            LIMIT_CODE
        );

        let oversized_source = "x".repeat(MAX_PATH_BYTES + 1);
        assert_eq!(
            inspect_revision3_exact_snapshot_v2_raw(&request(&oversized_source))["error"]["code"],
            LIMIT_CODE
        );
        assert_eq!(
            inspect_revision3_exact_snapshot_v2_raw(&request(""))["error"]["code"],
            SOURCE_INVALID_CODE
        );
        assert_eq!(
            inspect_revision3_exact_snapshot_v2_raw(&request("bad\0source"))["error"]["code"],
            SOURCE_INVALID_CODE
        );
    }

    #[test]
    fn exact_import_wire_rejects_unknown_duplicate_missing_and_wrong_shapes() {
        const SHA: &str = "1111111111111111111111111111111111111111111111111111111111111111";
        let cases = [
            format!(
                r#"{{"command":"{IMPORT_COMMAND}","payload":{{"source":"x","destination":"y","expected_archive":{{"byte_len":1,"sha256":"{SHA}"}},"future":true}}}}"#
            ),
            format!(
                r#"{{"command":"{IMPORT_COMMAND}","payload":{{"source":"x","source":"x","destination":"y","expected_archive":{{"byte_len":1,"sha256":"{SHA}"}}}}}}"#
            ),
            format!(
                r#"{{"command":"{IMPORT_COMMAND}","payload":{{"source":"x","expected_archive":{{"byte_len":1,"sha256":"{SHA}"}}}}}}"#
            ),
            format!(
                r#"{{"command":"{IMPORT_COMMAND}","payload":{{"source":"x","destination":"y","expected_archive":{{"byte_len":"1","sha256":"{SHA}"}}}}}}"#
            ),
            format!(
                r#"{{"command":"{IMPORT_COMMAND}","payload":{{"source":"x","destination":"y","expected_archive":{{"byte_len":1,"sha256":"{SHA}","future":true}}}}}}"#
            ),
            format!(
                r#"{{"command":"{COMMAND}","payload":{{"source":"x","destination":"y","expected_archive":{{"byte_len":1,"sha256":"{SHA}"}}}}}}"#
            ),
        ];
        for wire in cases {
            let response = import_revision3_exact_snapshot_v2_raw(&wire);
            assert_eq!(response["error"]["code"], REQUEST_INVALID_CODE, "{wire}");
        }
    }

    #[test]
    fn import_wire_paths_and_archive_seal_are_bounded_before_native_access() {
        let seal: ContentSeal = serde_json::from_value(json!({
            "byte_len": 1,
            "sha256": "2222222222222222222222222222222222222222222222222222222222222222",
        }))
        .unwrap();
        assert_eq!(
            import_revision3_exact_snapshot_v2_raw(&" ".repeat(MAX_IMPORT_WIRE_BYTES + 1))["error"]
                ["code"],
            LIMIT_CODE
        );
        assert_eq!(
            import_revision3_exact_snapshot_v2_raw(&import_request(
                "x",
                &"y".repeat(MAX_PATH_BYTES + 1),
                &seal,
            ))["error"]["code"],
            LIMIT_CODE
        );
        assert_eq!(
            import_revision3_exact_snapshot_v2_raw(&import_request("x", "", &seal))["error"]
                ["code"],
            DESTINATION_INVALID_CODE
        );
        assert_eq!(
            import_revision3_exact_snapshot_v2_raw(&import_request("same", "same", &seal))["error"]
                ["code"],
            DESTINATION_INVALID_CODE
        );
        let zero_seal: ContentSeal = serde_json::from_value(json!({
            "byte_len": 0,
            "sha256": "2222222222222222222222222222222222222222222222222222222222222222",
        }))
        .unwrap();
        assert_eq!(
            import_revision3_exact_snapshot_v2_raw(&import_request("x", "y", &zero_seal))["error"]
                ["code"],
            LIMIT_CODE
        );
    }

    #[test]
    fn publication_uncertainty_has_no_receipt_or_identity_fields() {
        let response =
            uncertain_import_response("source.goremod".to_owned(), "destination".to_owned());
        assert_eq!(response["ok"], true, "{response:#}");
        assert_eq!(response["outcome"], "publication_uncertain");
        assert_eq!(response["publication_status"], "publication_uncertain");
        assert_eq!(response["retry_safe"], false);
        assert_eq!(response["warning"]["code"], PUBLICATION_UNCERTAIN_CODE);
        assert_eq!(
            exact_keys(&response),
            BTreeSet::from([
                "ok",
                "outcome",
                "source",
                "destination",
                "format",
                "artifact_kind",
                "restore_status",
                "inspection_status",
                "import_status",
                "project_mutation",
                "session_adoption",
                "game_mutation",
                "save_mutation",
                "build_status",
                "deployment_status",
                "runtime_status",
                "publication_status",
                "retry_safe",
                "warning",
            ])
        );
        for forbidden in [
            "archive",
            "manifest",
            "project_id",
            "project_revision",
            "head_json",
            "closure",
        ] {
            assert!(response.get(forbidden).is_none(), "{forbidden}");
        }
    }

    #[test]
    #[cfg(windows)]
    fn v1_review_copy_has_one_distinct_non_restorable_error() {
        let (temp, store, head) = published_store();
        let source = temp.path().join("Review Copy.goremod");
        let publication = store
            .export_current_revision3_exact_snapshot_v1(&head, &source)
            .unwrap();
        assert!(matches!(
            publication,
            Revision3ExactSnapshotExportPublicationV1::Exported(_)
        ));

        let response = inspect_revision3_exact_snapshot_v2_raw(&request(&source.to_string_lossy()));
        assert_eq!(response["error"]["code"], UNSUPPORTED_REVIEW_COPY_CODE);
        assert!(!response["error"]["message"]
            .as_str()
            .unwrap()
            .contains(&source.to_string_lossy().to_string()));
    }

    #[test]
    #[cfg(windows)]
    fn corrupt_archive_is_closed_sanitized_and_read_only() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("private-corrupt.goremod");
        fs::write(&source, b"not a ZIP").unwrap();
        let before_bytes = fs::read(&source).unwrap();
        let before_tree = directory_tree(temp.path());

        let response = inspect_revision3_exact_snapshot_v2_raw(&request(&source.to_string_lossy()));

        assert_eq!(response["error"]["code"], ARCHIVE_INVALID_CODE);
        assert!(!response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("private-corrupt"));
        assert_eq!(fs::read(&source).unwrap(), before_bytes);
        assert_eq!(directory_tree(temp.path()), before_tree);
    }

    #[test]
    #[cfg(windows)]
    fn v2_success_matches_the_closed_dart_receipt_and_writes_nothing() {
        let fixture = exported_v2_fixture();
        let source = fixture.source.to_string_lossy().into_owned();
        let before_bytes = fs::read(&fixture.source).unwrap();
        let parent = fixture.source.parent().unwrap();
        let before_tree = directory_tree(parent);

        let response = inspect_revision3_exact_snapshot_v2_raw(&request(&source));
        let routed: Value = serde_json::from_str(&crate::execute_json(&request(&source))).unwrap();

        assert_eq!(response["ok"], true);
        assert_eq!(routed, response);
        assert_eq!(response["outcome"], "inspected_restorable_copy");
        assert_eq!(response["source"], source);
        assert_eq!(
            response["format"],
            REVISION3_EXACT_SNAPSHOT_IMPORT_FORMAT_V2
        );
        assert_eq!(
            response["artifact_kind"],
            REVISION3_EXACT_SNAPSHOT_IMPORT_ARTIFACT_KIND_V2
        );
        assert_eq!(
            response["restore_status"],
            REVISION3_EXACT_SNAPSHOT_IMPORT_RESTORE_STATUS_V2
        );
        assert_eq!(response["archive"], json!(fixture.receipt.archive));
        assert_eq!(
            response["manifest"],
            json!({
                "relative_name": REVISION3_EXACT_SNAPSHOT_IMPORT_MANIFEST_FILE_V2,
                "byte_len": fixture.receipt.manifest.byte_len,
                "sha256": fixture.receipt.manifest.sha256,
            })
        );
        assert_eq!(
            response["project_id"],
            fixture.receipt.project_id.to_string()
        );
        assert_eq!(
            response["project_revision"],
            fixture.receipt.project_revision
        );
        assert_eq!(
            response["head_json"],
            serde_json::to_string(&fixture.receipt.head).unwrap()
        );
        assert_eq!(response["closure"], json!(fixture.receipt.closure));
        assert_eq!(response["inspection_status"], "verified_exact");
        assert_eq!(response["import_status"], "not_performed");
        assert_eq!(response["project_mutation"], "not_performed");
        assert_eq!(response["game_mutation"], "not_performed");
        assert_eq!(response["save_mutation"], "not_performed");
        assert_eq!(response["build_status"], "not_performed");
        assert_eq!(response["deployment_status"], "not_performed");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["publication_status"], "not_supported");
        assert_eq!(response["retry_safe"], true);

        let expected = BTreeSet::from([
            "ok",
            "outcome",
            "source",
            "format",
            "artifact_kind",
            "restore_status",
            "archive",
            "manifest",
            "project_id",
            "project_revision",
            "head_json",
            "closure",
            "inspection_status",
            "import_status",
            "project_mutation",
            "game_mutation",
            "save_mutation",
            "build_status",
            "deployment_status",
            "runtime_status",
            "publication_status",
            "retry_safe",
        ]);
        assert_eq!(exact_keys(&response), expected);
        assert_eq!(
            exact_keys(&response["archive"]),
            BTreeSet::from(["byte_len", "sha256"])
        );
        assert_eq!(
            exact_keys(&response["manifest"]),
            BTreeSet::from(["relative_name", "byte_len", "sha256"])
        );
        assert_eq!(
            exact_keys(&response["closure"]),
            BTreeSet::from([
                "snapshot_objects",
                "entity_objects",
                "asset_objects",
                "archive_entries",
                "uncompressed_bytes",
            ])
        );
        assert_eq!(fs::read(&fixture.source).unwrap(), before_bytes);
        assert_eq!(directory_tree(parent), before_tree);

        let mut impossible = fixture.receipt.clone();
        impossible.archive.byte_len = impossible.closure.uncompressed_bytes;
        assert_eq!(
            inspection_response(impossible, source).unwrap_err().code,
            INVARIANT_CODE
        );
    }

    #[test]
    #[cfg(windows)]
    fn v2_destination_import_routes_and_reopens_one_exact_arbitrary_named_project() {
        let fixture = exported_v2_fixture();
        let source = fixture.source.to_string_lossy().into_owned();
        let destination_path = fixture
            .source
            .parent()
            .unwrap()
            .join("Restored project without required suffix");
        let destination = destination_path.to_string_lossy().into_owned();
        let source_before = fs::read(&fixture.source).unwrap();
        let wire = import_request(&source, &destination, &fixture.receipt.archive);

        let response: Value = serde_json::from_str(&crate::execute_json(&wire)).unwrap();

        assert_eq!(response["ok"], true, "{response:#}");
        assert_eq!(response["outcome"], "imported");
        assert_eq!(response["source"], source);
        assert_eq!(response["destination"], destination);
        assert_eq!(response["archive"], json!(fixture.receipt.archive));
        assert_eq!(
            response["manifest"],
            json!({
                "relative_name": REVISION3_EXACT_SNAPSHOT_IMPORT_MANIFEST_FILE_V2,
                "byte_len": fixture.receipt.manifest.byte_len,
                "sha256": fixture.receipt.manifest.sha256,
            })
        );
        assert_eq!(
            response["project_id"],
            fixture.receipt.project_id.to_string()
        );
        assert_eq!(
            response["project_revision"],
            fixture.receipt.project_revision
        );
        assert_eq!(
            response["head_json"],
            serde_json::to_string(&fixture.receipt.head).unwrap()
        );
        assert_eq!(response["closure"], json!(fixture.receipt.closure));
        assert_eq!(response["inspection_status"], "verified_exact");
        assert_eq!(response["import_status"], "materialized");
        assert_eq!(response["project_mutation"], "materialized");
        assert_eq!(response["session_adoption"], "not_performed");
        assert_eq!(response["game_mutation"], "not_performed");
        assert_eq!(response["save_mutation"], "not_performed");
        assert_eq!(response["build_status"], "not_performed");
        assert_eq!(response["deployment_status"], "not_performed");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["publication_status"], "published");
        assert_eq!(response["retry_safe"], false);
        assert!(response["warning"].is_null());
        assert_eq!(
            exact_keys(&response),
            BTreeSet::from([
                "ok",
                "outcome",
                "source",
                "destination",
                "format",
                "artifact_kind",
                "restore_status",
                "archive",
                "manifest",
                "project_id",
                "project_revision",
                "head_json",
                "closure",
                "inspection_status",
                "import_status",
                "project_mutation",
                "session_adoption",
                "game_mutation",
                "save_mutation",
                "build_status",
                "deployment_status",
                "runtime_status",
                "publication_status",
                "retry_safe",
                "warning",
            ])
        );
        assert_eq!(fs::read(&fixture.source).unwrap(), source_before);

        let store =
            WorkingProjectStore::open_existing(&destination_path, WorkingStoreLimits::default())
                .unwrap();
        let opened = store
            .open_current_revision3(AssetVerification::Full)
            .unwrap();
        assert_eq!(opened.head, fixture.receipt.head);
        assert_eq!(opened.project.project_id, fixture.receipt.project_id);
        assert_eq!(opened.project.revision, fixture.receipt.project_revision);
    }

    #[test]
    #[cfg(windows)]
    fn v2_destination_import_cas_mismatch_and_existing_destination_publish_nothing() {
        let fixture = exported_v2_fixture();
        let source = fixture.source.to_string_lossy().into_owned();
        let parent = fixture.source.parent().unwrap();
        let before_tree = directory_tree(parent);
        let source_before = fs::read(&fixture.source).unwrap();
        let destination_path = parent.join("CAS mismatch destination");
        let destination = destination_path.to_string_lossy().into_owned();
        let mut wrong_archive = fixture.receipt.archive.clone();
        wrong_archive.byte_len += 1;

        let response = import_revision3_exact_snapshot_v2_raw(&import_request(
            &source,
            &destination,
            &wrong_archive,
        ));
        assert_eq!(response["error"]["code"], SOURCE_CHANGED_CODE);
        assert!(!destination_path.exists());
        assert_eq!(directory_tree(parent), before_tree);
        assert_eq!(fs::read(&fixture.source).unwrap(), source_before);

        fs::create_dir(&destination_path).unwrap();
        let sentinel = destination_path.join("keep.txt");
        fs::write(&sentinel, b"keep").unwrap();
        let response = import_revision3_exact_snapshot_v2_raw(&import_request(
            &source,
            &destination,
            &fixture.receipt.archive,
        ));
        assert_eq!(response["error"]["code"], DESTINATION_INVALID_CODE);
        assert_eq!(fs::read(&sentinel).unwrap(), b"keep");
        assert_eq!(fs::read_dir(&destination_path).unwrap().count(), 1);
        assert_eq!(fs::read(&fixture.source).unwrap(), source_before);
    }

    #[test]
    #[cfg(not(windows))]
    fn public_dispatch_reports_the_stable_platform_error_before_source_io() {
        let response: Value =
            serde_json::from_str(&crate::execute_json(&request("/not-opened.goremod"))).unwrap();
        assert_eq!(response["error"]["code"], PLATFORM_UNSUPPORTED_CODE);

        let temp = TempDir::new().unwrap();
        let destination = temp.path().join("must-not-be-created");
        let expected_archive: ContentSeal = serde_json::from_value(json!({
            "byte_len": 1,
            "sha256": "3333333333333333333333333333333333333333333333333333333333333333",
        }))
        .unwrap();
        let response: Value = serde_json::from_str(&crate::execute_json(&import_request(
            "/not-opened.goremod",
            &destination.to_string_lossy(),
            &expected_archive,
        )))
        .unwrap();
        assert_eq!(response["error"]["code"], PLATFORM_UNSUPPORTED_CODE);
        assert!(!destination.exists());
    }

    #[test]
    fn native_error_classes_map_to_stable_sanitized_codes() {
        let cases = [
            (
                Revision3ExactSnapshotInspectionErrorV2::UnsupportedPlatform,
                PLATFORM_UNSUPPORTED_CODE,
            ),
            (
                Revision3ExactSnapshotInspectionErrorV2::InvalidSource(
                    "C:\\private\\source.goremod".to_owned(),
                ),
                SOURCE_INVALID_CODE,
            ),
            (
                Revision3ExactSnapshotInspectionErrorV2::Limit {
                    kind: "secret kind",
                    actual: u64::MAX,
                    limit: 1,
                },
                LIMIT_CODE,
            ),
            (
                Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(
                    "C:\\private\\archive.goremod".to_owned(),
                ),
                ARCHIVE_INVALID_CODE,
            ),
            (
                Revision3ExactSnapshotInspectionErrorV2::InvalidManifest(
                    "private manifest member".to_owned(),
                ),
                MANIFEST_INVALID_CODE,
            ),
            (
                Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
                    "private Store object".to_owned(),
                ),
                CLOSURE_INVALID_CODE,
            ),
        ];
        for (error, code) in cases {
            let failure = map_inspection_error(error);
            assert_eq!(failure.code, code);
            assert!(!failure.message.contains("private"));
            assert!(failure.message.len() <= MAX_ERROR_MESSAGE_BYTES);
        }
        assert_eq!(
            map_inspection_error(Revision3ExactSnapshotInspectionErrorV2::UnsupportedReviewCopyV1)
                .code,
            UNSUPPORTED_REVIEW_COPY_CODE
        );

        let seal: ContentSeal = serde_json::from_value(json!({
            "byte_len": 1,
            "sha256": "4444444444444444444444444444444444444444444444444444444444444444",
        }))
        .unwrap();
        let import_cases = [
            (
                Revision3ExactSnapshotImportErrorV2::Inspection(
                    Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(
                        "C:\\LEAK_MARKER\\changed.goremod".to_owned(),
                    ),
                ),
                SOURCE_CHANGED_CODE,
            ),
            (
                Revision3ExactSnapshotImportErrorV2::ArchiveCasMismatch {
                    expected: seal.clone(),
                    actual: seal,
                },
                SOURCE_CHANGED_CODE,
            ),
            (
                Revision3ExactSnapshotImportErrorV2::InvalidDestination(
                    "C:\\LEAK_MARKER\\destination".to_owned(),
                ),
                DESTINATION_INVALID_CODE,
            ),
            (
                Revision3ExactSnapshotImportErrorV2::DestinationAlreadyExists,
                DESTINATION_INVALID_CODE,
            ),
            (
                Revision3ExactSnapshotImportErrorV2::Materialization(
                    "LEAK_MARKER materialization".to_owned(),
                ),
                MATERIALIZATION_FAILED_CODE,
            ),
            (
                Revision3ExactSnapshotImportErrorV2::CandidateVerification(
                    "LEAK_MARKER verification".to_owned(),
                ),
                VERIFICATION_FAILED_CODE,
            ),
            (
                Revision3ExactSnapshotImportErrorV2::Publication(
                    "LEAK_MARKER publication".to_owned(),
                ),
                PUBLICATION_FAILED_CODE,
            ),
            (
                Revision3ExactSnapshotImportErrorV2::StagingCleanup {
                    primary: Box::new(Revision3ExactSnapshotImportErrorV2::Materialization(
                        "LEAK_MARKER primary".to_owned(),
                    )),
                    cleanup: "LEAK_MARKER cleanup".to_owned(),
                },
                CLEANUP_FAILED_CODE,
            ),
        ];
        for (error, code) in import_cases {
            let failure = map_import_error(error);
            assert_eq!(failure.code, code);
            assert!(!failure.message.contains("LEAK_MARKER"));
            assert!(failure.message.len() <= MAX_ERROR_MESSAGE_BYTES);
        }
        assert_eq!(
            map_import_error(Revision3ExactSnapshotImportErrorV2::Inspection(
                Revision3ExactSnapshotInspectionErrorV2::UnsupportedPlatform,
            ))
            .code,
            PLATFORM_UNSUPPORTED_CODE
        );

        let bounded = Failure::new("TEST", "é".repeat(MAX_ERROR_MESSAGE_BYTES));
        assert!(bounded.message.len() <= MAX_ERROR_MESSAGE_BYTES);
        assert!(bounded.message.ends_with("..."));
    }
}
