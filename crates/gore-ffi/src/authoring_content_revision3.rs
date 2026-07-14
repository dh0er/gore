//! Exact-current, read-only revision-3 content projection for Mod Studio.
//!
//! This command fully reopens the published project and its assets on both sides of projection,
//! requires the caller's exact head, and returns only the bounded semantic index. It grants no
//! mutation, publication, build, deployment, artifact, or runtime authority.

use std::path::Path;

use gore_authoring::{
    build_revision3_content_index_v1, AssetVerification, Revision3ContentIndexErrorV1,
    Revision3ContentIndexJsonErrorV1, Revision3EntityPayload, Revision3OriginRef, WorkingHead,
    WorkingProjectStore, WorkingStoreError, WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_read_revision3_content_index_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_WIRE_BYTES: usize = MAX_PATH_BYTES * 6 + MAX_HEAD_JSON_BYTES * 2 + 4 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadContentWirePayload {
    expected_head_json: String,
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

pub(super) fn read_revision3_content_index_v1_raw(input: &str) -> Value {
    read_revision3_content_index_v1_inner(input).unwrap_or_else(Failure::response)
}

fn read_revision3_content_index_v1_inner(input: &str) -> Result<Value, Failure> {
    let payload: ReadContentWirePayload = parse_exact_wire(input)?;
    validate_path(&payload.root)?;
    let expected_head = parse_canonical_head(&payload.expected_head_json)?;

    let store = WorkingProjectStore::open_existing(Path::new(&payload.root), ffi_store_limits())
        .map_err(map_store_error)?;
    let before = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if before.head != expected_head {
        return Err(head_conflict());
    }
    validate_signed_wire_values(&before.project)?;

    let index = build_revision3_content_index_v1(&before.project).map_err(map_index_error)?;
    let index_json = index.to_canonical_json().map_err(map_index_json_error)?;

    // The first full open proved the projected assets. The second full open closes the mutable
    // read window and rejects head/project or asset drift before returning UI evidence.
    let after = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if after.head != expected_head || after.project != before.project {
        return Err(head_conflict());
    }

    let head_json = serde_json::to_string(&after.head).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_CONTENT_INVARIANT",
            "revision-3 content head could not be serialized",
        )
    })?;
    if head_json != payload.expected_head_json {
        return Err(Failure::new(
            "AUTHORING_REVISION3_CONTENT_HEAD_INVALID",
            "expected_head_json is not in exact canonical form",
        ));
    }

    enforce_response_budget(json!({
        "ok": true,
        "head_json": head_json,
        "project_id": after.project.project_id.to_string(),
        "project_revision": after.project.revision,
        "index_json": index_json,
        "content_authority": "read_only_exact_current_project",
        "build_status": "not_evaluated",
        "runtime_status": "runtime_unqualified",
        "publication_status": "not_applicable",
    }))
}

fn parse_exact_wire<P: DeserializeOwned>(input: &str) -> Result<P, Failure> {
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_CONTENT_INPUT_LIMIT",
            format!("revision-3 content request exceeds the {MAX_WIRE_BYTES}-byte wire limit"),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
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
            "AUTHORING_REVISION3_CONTENT_HEAD_INVALID",
            "expected_head_json is empty or exceeds its bounded transport limit",
        ));
    }
    let head: WorkingHead = serde_json::from_str(input).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_CONTENT_HEAD_INVALID",
            "expected_head_json is not one closed revision-3 working head",
        )
    })?;
    let canonical = serde_json::to_string(&head).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_CONTENT_INVARIANT",
            "revision-3 content head could not be serialized",
        )
    })?;
    if canonical != input {
        return Err(Failure::new(
            "AUTHORING_REVISION3_CONTENT_HEAD_INVALID",
            "expected_head_json is not duplicate-free canonical JSON",
        ));
    }
    Ok(head)
}

fn validate_signed_wire_values(project: &gore_authoring::ProjectRevision3) -> Result<(), Failure> {
    signed_wire_u64(project.revision)?;
    signed_wire_u64(project.target.executable.byte_len)?;
    for meta in project.asset_store.assets.values() {
        signed_wire_u64(meta.byte_len)?;
    }
    for entity in project.entities.values() {
        signed_wire_u64(entity.revision)?;
        match &entity.origin {
            Revision3OriginRef::Vanilla {
                generation,
                source_seal,
                ..
            } => {
                signed_wire_u64(generation.executable.byte_len)?;
                signed_wire_u64(source_seal.byte_len)?;
            }
            Revision3OriginRef::Imported { source_seal, .. } => {
                signed_wire_u64(source_seal.byte_len)?;
            }
            Revision3OriginRef::New { .. } | Revision3OriginRef::Generated { .. } => {}
        }
        match &entity.payload {
            Revision3EntityPayload::VoiceTake(take) => signed_wire_u64(take.asset.byte_len)?,
            Revision3EntityPayload::QuestDraft(quest) => {
                signed_wire_u64(quest.input.collision_catalog.artifact.byte_len)?;
            }
            Revision3EntityPayload::LocalizationEntry(_)
            | Revision3EntityPayload::DialogLine(_)
            | Revision3EntityPayload::VoiceSlot(_)
            | Revision3EntityPayload::NpcDraft(_)
            | Revision3EntityPayload::ScriptModule(_) => {}
        }
    }
    Ok(())
}

fn signed_wire_u64(value: u64) -> Result<(), Failure> {
    if value > i64::MAX as u64 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_CONTENT_RESPONSE_LIMIT",
            "revision-3 content contains an integer outside the signed wire range",
        ));
    }
    Ok(())
}

fn enforce_response_budget(response: Value) -> Result<Value, Failure> {
    let bytes = serde_json::to_vec(&response).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_CONTENT_INVARIANT",
            "revision-3 content response could not be serialized",
        )
    })?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_CONTENT_RESPONSE_LIMIT",
            "revision-3 content response exceeds its bounded transport budget",
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
        "AUTHORING_REVISION3_CONTENT_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and exactly expected_head_json and root",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_CONTENT_HEAD_CONFLICT",
        "the published revision-3 head changed or differs from the caller's exact head",
    )
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match &error {
        WorkingStoreError::InvalidLimits(_) => "AUTHORING_REVISION3_CONTENT_STORE_LIMITS_INVALID",
        WorkingStoreError::MissingRoot(_) => "AUTHORING_REVISION3_CONTENT_STORE_ROOT_MISSING",
        WorkingStoreError::UnsafePath { .. } => "AUTHORING_REVISION3_CONTENT_STORE_PATH_UNSAFE",
        WorkingStoreError::LimitExceeded { .. } => "AUTHORING_REVISION3_CONTENT_STORE_LIMIT",
        WorkingStoreError::HeadConflict { .. } => "AUTHORING_REVISION3_CONTENT_HEAD_CONFLICT",
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_CONTENT_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => "AUTHORING_REVISION3_CONTENT_STORE_OBJECT_MISSING",
        WorkingStoreError::SealMismatch { .. } => "AUTHORING_REVISION3_CONTENT_STORE_SEAL_MISMATCH",
        WorkingStoreError::Collision { .. } => "AUTHORING_REVISION3_CONTENT_STORE_COLLISION",
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_CONTENT_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_CONTENT_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_CONTENT_STORE_IO"
        }
    };
    Failure::new(code, "the revision-3 working store read failed")
}

fn map_index_error(error: Revision3ContentIndexErrorV1) -> Failure {
    match error {
        Revision3ContentIndexErrorV1::TooManyReferences { .. } => Failure::new(
            "AUTHORING_REVISION3_CONTENT_RESPONSE_LIMIT",
            "revision-3 content reference graph exceeds its bounded response limit",
        ),
        Revision3ContentIndexErrorV1::InvalidProject(_) => Failure::new(
            "AUTHORING_REVISION3_CONTENT_STORE_INVARIANT",
            "revision-3 content projection rejected the fully reopened current project",
        ),
    }
}

fn map_index_json_error(error: Revision3ContentIndexJsonErrorV1) -> Failure {
    match error {
        Revision3ContentIndexJsonErrorV1::TooLarge { .. } => Failure::new(
            "AUTHORING_REVISION3_CONTENT_RESPONSE_LIMIT",
            "revision-3 content index exceeds its bounded response limit",
        ),
        Revision3ContentIndexJsonErrorV1::Serialize(_)
        | Revision3ContentIndexJsonErrorV1::NonUtf8Serialization => Failure::new(
            "AUTHORING_REVISION3_CONTENT_INVARIANT",
            "revision-3 content index could not be serialized",
        ),
    }
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

    use gore_authoring::{
        AssetStoreIndex, ContentSeal, EntityId, FormatV2, GameGenerationAnchor, ProjectId,
        ProjectMeta, ProjectRevision3, Revision3Entity, Revision3EntityPayload, Revision3OriginRef,
        SchemaRevisionV3, Sha256Digest,
    };
    use serde_json::Value;
    use tempfile::TempDir;

    use super::*;

    fn project(revision: u64) -> ProjectRevision3 {
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([1; 16]),
            revision,
            meta: ProjectMeta {
                name: "Content fixture".to_owned(),
                version: "0.1.0".to_owned(),
                author: "GORE".to_owned(),
            },
            target: GameGenerationAnchor {
                executable: ContentSeal {
                    byte_len: 4,
                    sha256: Sha256Digest::from_bytes([2; 32]),
                },
            },
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn published_store(revision: u64) -> (TempDir, String) {
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let prepared = store
            .prepare_revision3_checkpoint(None, &project(revision))
            .unwrap();
        fs::write(temp.path().join("gore-project.json"), &prepared.head_bytes).unwrap();
        (
            temp,
            String::from_utf8(prepared.head_bytes).expect("canonical head UTF-8"),
        )
    }

    fn call(root: &Path, head_json: &str) -> Value {
        read_revision3_content_index_v1_raw(
            &serde_json::to_string(&json!({
                "command": COMMAND,
                "payload": {
                    "expected_head_json": head_json,
                    "root": root.to_string_lossy(),
                },
            }))
            .unwrap(),
        )
    }

    #[test]
    fn exact_current_project_returns_one_closed_read_only_index() {
        let (temp, head_json) = published_store(7);
        let response = call(temp.path(), &head_json);
        assert_eq!(response["ok"], true);
        assert_eq!(response["head_json"], head_json);
        assert_eq!(response["project_revision"], 7);
        assert_eq!(
            response["content_authority"],
            "read_only_exact_current_project"
        );
        assert_eq!(response["build_status"], "not_evaluated");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["publication_status"], "not_applicable");

        let index: Value = serde_json::from_str(response["index_json"].as_str().unwrap()).unwrap();
        assert_eq!(index["schema_revision"], 1);
        assert_eq!(index["project_revision"], 7);
        assert_eq!(index["entities"], json!([]));
        assert_eq!(index["assets"], json!([]));
    }

    #[test]
    fn stale_or_noncanonical_head_fails_closed() {
        let (temp, head_json) = published_store(1);
        let stale = head_json.replace("\"byte_len\":", "\"byte_len\":999");
        assert_eq!(
            call(temp.path(), &stale)["error"]["code"],
            "AUTHORING_REVISION3_CONTENT_HEAD_CONFLICT"
        );
        assert_eq!(
            call(temp.path(), &format!(" {head_json}"))["error"]["code"],
            "AUTHORING_REVISION3_CONTENT_HEAD_INVALID"
        );
        let duplicate = head_json.replacen("{", "{\"store_format\":1,", 1);
        assert_eq!(
            call(temp.path(), &duplicate)["error"]["code"],
            "AUTHORING_REVISION3_CONTENT_HEAD_INVALID"
        );
    }

    #[test]
    fn outer_wire_rejects_duplicates_unknowns_wrong_command_and_oversize() {
        for raw in [
            format!(
                "{{\"command\":\"{COMMAND}\",\"command\":\"{COMMAND}\",\"payload\":{{\"expected_head_json\":\"x\",\"root\":\"x\"}}}}"
            ),
            format!(
                "{{\"command\":\"{COMMAND}\",\"payload\":{{\"expected_head_json\":\"x\",\"root\":\"x\",\"extra\":false}}}}"
            ),
            "{\"command\":\"wrong\",\"payload\":{\"expected_head_json\":\"x\",\"root\":\"x\"}}".to_owned(),
            format!(
                "{{\"command\":\"{COMMAND}\",\"payload\":{{\"expected_head_json\":\"x\",\"root\":\"{}\"}}}}",
                "x".repeat(MAX_PATH_BYTES + 1)
            ),
        ] {
            assert_eq!(
                read_revision3_content_index_v1_raw(&raw)["error"]["code"],
                "AUTHORING_REVISION3_CONTENT_REQUEST_INVALID"
            );
        }
        assert_eq!(
            read_revision3_content_index_v1_raw(&" ".repeat(MAX_WIRE_BYTES + 1))["error"]["code"],
            "AUTHORING_REVISION3_CONTENT_INPUT_LIMIT"
        );
    }

    #[test]
    fn revisions_outside_signed_wire_range_are_rejected() {
        let (temp, head_json) = published_store(i64::MAX as u64 + 1);
        assert_eq!(
            call(temp.path(), &head_json)["error"]["code"],
            "AUTHORING_REVISION3_CONTENT_RESPONSE_LIMIT"
        );
    }

    #[test]
    fn every_projected_u64_must_fit_the_signed_wire() {
        let mut oversized_target = project(0);
        oversized_target.target.executable.byte_len = u64::MAX;
        assert_eq!(
            validate_signed_wire_values(&oversized_target)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_CONTENT_RESPONSE_LIMIT"
        );

        let mut oversized_origin = project(0);
        let entity_id = EntityId::from_bytes([9; 16]);
        oversized_origin.entities.insert(
            entity_id,
            Revision3Entity {
                id: entity_id,
                display_name: "Imported localization".to_owned(),
                origin: Revision3OriginRef::Imported {
                    importer: "fixture".to_owned(),
                    source_seal: ContentSeal {
                        byte_len: u64::MAX,
                        sha256: Sha256Digest::from_bytes([8; 32]),
                    },
                    external_identity: None,
                },
                revision: 0,
                payload: Revision3EntityPayload::LocalizationEntry(
                    gore_authoring::model_revision3::LocalizationEntry {
                        loc_id: "LOC_IMPORTED".to_owned(),
                        texts: BTreeMap::new(),
                    },
                ),
            },
        );
        assert_eq!(
            validate_signed_wire_values(&oversized_origin)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_CONTENT_RESPONSE_LIMIT"
        );
    }

    #[test]
    fn deterministic_asset_inconsistency_is_not_misreported_as_io() {
        assert_eq!(
            map_store_error(WorkingStoreError::InvalidOgg("fixture".to_owned())).code,
            "AUTHORING_REVISION3_CONTENT_STORE_INVARIANT"
        );
        assert_eq!(
            map_store_error(WorkingStoreError::Io(std::io::Error::other("fixture"))).code,
            "AUTHORING_REVISION3_CONTENT_STORE_IO"
        );
    }
}
