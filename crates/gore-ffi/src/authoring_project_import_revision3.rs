//! Read-only inspection of one untrusted restorable managed revision-3 snapshot.
//!
//! This command accepts only a source path. Native authoring code opens and
//! verifies the complete V2 archive through one retained handle. This FFI lane
//! never accepts a destination or Store root and cannot extract, adopt, mutate,
//! restore, or publish anything.

use std::path::Path;

use gore_authoring::{
    inspect_revision3_exact_snapshot_v2, Revision3ExactSnapshotInspectionErrorV2,
    Revision3ExactSnapshotInspectionV2, REVISION3_EXACT_SNAPSHOT_IMPORT_ARTIFACT_KIND_V2,
    REVISION3_EXACT_SNAPSHOT_IMPORT_FORMAT_V2, REVISION3_EXACT_SNAPSHOT_IMPORT_MANIFEST_FILE_V2,
    REVISION3_EXACT_SNAPSHOT_IMPORT_RESTORE_STATUS_V2,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_inspect_revision3_exact_snapshot_v2";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_WIRE_BYTES: usize = MAX_PATH_BYTES * 6 + 1024;
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

fn inspect_revision3_exact_snapshot_v2_inner(input: &str) -> Result<Value, Failure> {
    let payload = parse_exact_wire(input)?;
    let source = payload.source;
    let inspection =
        inspect_revision3_exact_snapshot_v2(Path::new(&source)).map_err(map_inspection_error)?;
    inspection_response(inspection, source)
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

fn invalid_request() -> Failure {
    Failure::new(
        REQUEST_INVALID_CODE,
        "managed snapshot inspection request must contain only one exact command and source payload",
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
    #[cfg(windows)]
    use std::collections::BTreeSet;
    #[cfg(windows)]
    use std::fs;
    #[cfg(windows)]
    use std::path::{Path, PathBuf};

    use gore_authoring::Revision3ExactSnapshotInspectionErrorV2;
    #[cfg(windows)]
    use gore_authoring::{
        ProjectRevision3, Revision3ExactSnapshotExportPublicationV1,
        Revision3ExactSnapshotExportPublicationV2, Revision3ExactSnapshotInspectionV2, WorkingHead,
        WorkingProjectStore, WorkingStoreLimits,
    };
    use serde_json::json;
    #[cfg(windows)]
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

    #[cfg(windows)]
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
    #[cfg(not(windows))]
    fn public_dispatch_reports_the_stable_platform_error_before_source_io() {
        let response: Value =
            serde_json::from_str(&crate::execute_json(&request("/not-opened.goremod"))).unwrap();
        assert_eq!(response["error"]["code"], PLATFORM_UNSUPPORTED_CODE);
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

        let bounded = Failure::new("TEST", "é".repeat(MAX_ERROR_MESSAGE_BYTES));
        assert!(bounded.message.len() <= MAX_ERROR_MESSAGE_BYTES);
        assert!(bounded.message.ends_with("..."));
    }
}
