//! Exact-current, read-only source/readiness inspection for one managed revision-3 NPC Draft.
//!
//! The client supplies only the selected working Store, exact current head, and NPC identity.
//! Native code fully opens the current project, verifies and regenerates the selected
//! NPC/ScriptModule closure from its persisted parent provenance, and returns one bounded sealed
//! plan. This route never writes the Store, touches a game installation or save, compiles source,
//! builds, spawns, deploys, publishes, or grants reusable authority.

use std::path::Path;

use gore_authoring::{
    AssetVerification, ContentSeal, EntityId, ProjectRevision3, Revision3EntityKind, Sha256Digest,
    WorkingHead, WorkingProjectStore, WorkingStoreError, WorkingStoreLimits,
    MAX_PROJECT_JSON_BYTES,
};
use gore_story_build::revision3_npc::{
    build_revision3_npc_source_inspection_plan_v1, NpcInspectionBuildStatusV1,
    NpcInspectionCompilerStatusV1, NpcInspectionPublicationStatusV1,
    NpcInspectionRuntimeQualificationV1, NpcInspectionScopeV1, NpcInspectionSourceStatusV1,
    NpcInspectionSpawnStatusV1, Revision3NpcInspectionErrorV1, Revision3NpcSourceInspectionPlanV1,
    MAX_REVISION3_NPC_INSPECTION_PLAN_JSON_BYTES,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

use crate::err;

pub(super) const COMMAND: &str = "authoring_store_inspect_revision3_npc_source_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const NPC_ID_BYTES: usize = 32;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
// The canonical plan is returned as a nested JSON string. Escaping a canonical JSON string can at
// most double its byte length; the remaining response fields fit comfortably in the reserve.
const MAX_RESPONSE_BYTES: usize = MAX_REVISION3_NPC_INSPECTION_PLAN_JSON_BYTES * 2 + 1024 * 1024;
// A path may expand to six bytes per source byte when represented as JSON. The canonical head and
// fixed-width ID have a tighter two-byte bound. Keep this far below the global transport ceiling.
const MAX_WIRE_BYTES: usize =
    MAX_PATH_BYTES * 6 + MAX_HEAD_JSON_BYTES * 2 + NPC_ID_BYTES * 2 + 4 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectNpcWirePayload {
    expected_head_json: String,
    npc_id: String,
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

pub(super) fn inspect_revision3_npc_source_v1_raw(input: &str) -> Value {
    inspect_revision3_npc_source_v1_inner(input).unwrap_or_else(Failure::response)
}

fn inspect_revision3_npc_source_v1_inner(input: &str) -> Result<Value, Failure> {
    let payload: InspectNpcWirePayload = parse_exact_wire(input)?;
    validate_path(&payload.root)?;
    let expected_head = parse_canonical_head(&payload.expected_head_json)?;
    let npc_id = parse_npc_id(&payload.npc_id)?;

    let store = WorkingProjectStore::open_existing(Path::new(&payload.root), ffi_store_limits())
        .map_err(map_store_error)?;
    let before = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if before.head != expected_head {
        return Err(head_conflict());
    }
    validate_signed_wire_project(&before.project)?;
    let canonical_project_json = before.project.to_canonical_json().map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_INSPECTION_INVARIANT",
            "the fully opened revision-3 project could not be canonicalized",
        )
    })?;
    let project_seal = seal_bytes(canonical_project_json.as_bytes());

    let plan = build_revision3_npc_source_inspection_plan_v1(&canonical_project_json, npc_id)
        .map_err(map_inspection_error)?;
    plan.verify_against_project(&canonical_project_json)
        .map_err(map_inspection_error)?;
    validate_closed_plan(&plan, &before.project, &project_seal, npc_id)?;
    let plan_json = plan.to_canonical_json().map_err(map_inspection_error)?;
    let reopened =
        Revision3NpcSourceInspectionPlanV1::from_json(&plan_json).map_err(map_inspection_error)?;
    if reopened != plan {
        return Err(invariant_failure());
    }
    let plan_seal = plan.content_seal().map_err(map_inspection_error)?;

    // A second full open closes the mutable Store window. It proves that the fixed head, canonical
    // project, and every referenced Store object remained exact throughout plan construction.
    let after = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if after.head != expected_head || after.project != before.project {
        return Err(head_conflict());
    }
    let head_json = serde_json::to_string(&after.head).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_INSPECTION_INVARIANT",
            "the exact revision-3 working head could not be serialized",
        )
    })?;
    if head_json != payload.expected_head_json {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_INSPECTION_HEAD_INVALID",
            "expected_head_json is not in exact canonical form",
        ));
    }

    enforce_response_budget(json!({
        "ok": true,
        "outcome": "inspection_only",
        "head_json": head_json,
        "project_id": after.project.project_id.to_string(),
        "project_revision": after.project.revision,
        "project_seal": project_seal,
        "npc_id": npc_id.to_string(),
        "plan_json": plan_json,
        "plan_seal": plan_seal,
        "scope": "source_readiness_inspection_only",
        "source_status": "persisted_and_regenerated_exact",
        "compiler_status": "not_run",
        "build_status": "blocked",
        "runtime_qualification": "runtime_unqualified",
        "spawn_status": "not_supported",
        "publication_status": "not_supported",
    }))
}

fn parse_exact_wire<P: DeserializeOwned>(input: &str) -> Result<P, Failure> {
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_INSPECTION_INPUT_LIMIT",
            format!("NPC inspection request exceeds the {MAX_WIRE_BYTES}-byte wire limit"),
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
            "AUTHORING_REVISION3_NPC_INSPECTION_HEAD_INVALID",
            "expected_head_json is empty or exceeds its bounded transport limit",
        ));
    }
    let head: WorkingHead = serde_json::from_str(input).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_INSPECTION_HEAD_INVALID",
            "expected_head_json is not one closed working head",
        )
    })?;
    let canonical = serde_json::to_string(&head).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_INSPECTION_INVARIANT",
            "the exact revision-3 working head could not be serialized",
        )
    })?;
    if canonical != input {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_INSPECTION_HEAD_INVALID",
            "expected_head_json is not duplicate-free canonical JSON",
        ));
    }
    Ok(head)
}

fn parse_npc_id(input: &str) -> Result<EntityId, Failure> {
    if input.len() != NPC_ID_BYTES {
        return Err(invalid_request());
    }
    input.parse().map_err(|_| invalid_request())
}

fn validate_closed_plan(
    plan: &Revision3NpcSourceInspectionPlanV1,
    project: &ProjectRevision3,
    project_seal: &ContentSeal,
    npc_id: EntityId,
) -> Result<(), Failure> {
    if plan.format() != "revision3_npc_source_inspection_plan"
        || plan.schema_revision() != 1
        || plan.scope() != NpcInspectionScopeV1::SourceReadinessInspectionOnly
        || plan.source_status() != NpcInspectionSourceStatusV1::PersistedAndRegeneratedExact
        || plan.compiler_status() != NpcInspectionCompilerStatusV1::NotRun
        || plan.build_status() != NpcInspectionBuildStatusV1::Blocked
        || plan.runtime_qualification() != NpcInspectionRuntimeQualificationV1::RuntimeUnqualified
        || plan.spawn_status() != NpcInspectionSpawnStatusV1::NotSupported
        || plan.publication_status() != NpcInspectionPublicationStatusV1::NotSupported
        || plan.provenance().project_id() != project.project_id
        || plan.provenance().project_revision() != project.revision
        || plan.provenance().target() != &project.target
        || plan.provenance().canonical_project() != project_seal
        || plan.npc().reference().project_id != project.project_id
        || plan.npc().reference().id != npc_id
        || plan.npc().reference().expected_kind != Revision3EntityKind::NpcDraft
        || plan.npc().script_module() != plan.module().reference()
        || plan.module().reference().project_id != project.project_id
        || plan.module().reference().expected_kind != Revision3EntityKind::ScriptModule
        || plan.diagnostics().len() != 4
        || plan.diagnostics().iter().any(|item| !item.blocks_build())
    {
        return Err(inspection_failed());
    }

    for value in [
        project.revision,
        plan.provenance().project_revision(),
        plan.npc().entity_revision(),
        plan.module().entity_revision(),
    ] {
        signed_wire_u64(value)?;
    }
    for seal in [
        &project.target.executable,
        plan.provenance().canonical_project(),
        &plan.provenance().target().executable,
        plan.npc().input_seal(),
        plan.module().persisted_source(),
        &plan.npc().input().target.executable,
        &plan.npc().input().parent_character_definition.source_seal,
        &plan.npc().input().parent_ai_agent_config.source_seal,
        &plan.npc().input().parent_spawn_definition.source_seal,
    ] {
        signed_wire_u64(seal.byte_len)?;
    }
    Ok(())
}

fn validate_signed_wire_project(project: &ProjectRevision3) -> Result<(), Failure> {
    signed_wire_u64(project.revision)?;
    signed_wire_u64(project.target.executable.byte_len)?;
    for asset in project.asset_store.assets.values() {
        signed_wire_u64(asset.byte_len)?;
    }
    Ok(())
}

fn signed_wire_u64(value: u64) -> Result<(), Failure> {
    if value > i64::MAX as u64 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_INSPECTION_RESPONSE_LIMIT",
            "NPC inspection contains an integer outside the signed wire range",
        ));
    }
    Ok(())
}

fn seal_bytes(bytes: &[u8]) -> ContentSeal {
    ContentSeal {
        byte_len: bytes.len() as u64,
        sha256: Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
    }
}

fn enforce_response_budget(response: Value) -> Result<Value, Failure> {
    let encoded = serde_json::to_vec(&response).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_INSPECTION_INVARIANT",
            "NPC inspection response could not be serialized",
        )
    })?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_INSPECTION_RESPONSE_LIMIT",
            "NPC inspection response exceeds its bounded transport budget",
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
        "AUTHORING_REVISION3_NPC_INSPECTION_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and exactly expected_head_json, npc_id, and root",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_NPC_INSPECTION_HEAD_CONFLICT",
        "the published revision-3 head changed or differs from the caller's exact head",
    )
}

fn inspection_failed() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_NPC_INSPECTION_FAILED",
        "the exact read-only NPC source/readiness inspection could not be produced",
    )
}

fn invariant_failure() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_NPC_INSPECTION_INVARIANT",
        "the native NPC source/readiness inspection failed an internal invariant",
    )
}

fn map_inspection_error(error: Revision3NpcInspectionErrorV1) -> Failure {
    use Revision3NpcInspectionErrorV1 as E;
    match error {
        E::ProjectJsonTooLarge { .. }
        | E::PlanFieldTooLarge { .. }
        | E::PlanJsonTooLarge { .. } => Failure::new(
            "AUTHORING_REVISION3_NPC_INSPECTION_RESPONSE_LIMIT",
            "NPC inspection exceeds its bounded resource limit",
        ),
        E::MissingNpc(_) | E::NotAnNpc(_) => Failure::new(
            "AUTHORING_REVISION3_NPC_INSPECTION_NPC_INVALID",
            "npc_id does not identify one managed revision-3 NPC Draft",
        ),
        E::InvalidProjectDocument(_)
        | E::NonCanonicalProjectJson
        | E::Revision3Required
        | E::ForeignGenerator { .. }
        | E::ForeignGeneration { .. }
        | E::MissingScriptModule { .. }
        | E::OwnerMismatch { .. }
        | E::PersistedSourceSealMismatch { .. }
        | E::InputFingerprintMismatch { .. }
        | E::PersistedModuleDrift { .. }
        | E::RegenerateNpc(_) => Failure::new(
            "AUTHORING_REVISION3_NPC_INSPECTION_PROJECT_INVALID",
            "the fully opened project is not a valid managed NPC inspection source",
        ),
        E::SerializeProject(_)
        | E::SerializeNpcInput(_)
        | E::SerializeEntityEnvelope { .. }
        | E::SerializePlan(_)
        | E::InvalidPlanJson(_)
        | E::NonCanonicalPlanJson
        | E::PlanInvariant(_)
        | E::PlanProjectBindingMismatch => invariant_failure(),
    }
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match &error {
        WorkingStoreError::InvalidLimits(_) => {
            "AUTHORING_REVISION3_NPC_INSPECTION_STORE_LIMITS_INVALID"
        }
        WorkingStoreError::MissingRoot(_) => {
            "AUTHORING_REVISION3_NPC_INSPECTION_STORE_ROOT_MISSING"
        }
        WorkingStoreError::UnsafePath { .. } => {
            "AUTHORING_REVISION3_NPC_INSPECTION_STORE_PATH_UNSAFE"
        }
        WorkingStoreError::LimitExceeded { .. } => "AUTHORING_REVISION3_NPC_INSPECTION_STORE_LIMIT",
        WorkingStoreError::HeadConflict { .. } => {
            "AUTHORING_REVISION3_NPC_INSPECTION_HEAD_CONFLICT"
        }
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_NPC_INSPECTION_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => {
            "AUTHORING_REVISION3_NPC_INSPECTION_STORE_OBJECT_MISSING"
        }
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_NPC_INSPECTION_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => "AUTHORING_REVISION3_NPC_INSPECTION_STORE_COLLISION",
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_NPC_INSPECTION_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_NPC_INSPECTION_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_NPC_INSPECTION_STORE_IO"
        }
    };
    Failure::new(code, "the revision-3 working Store read failed")
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

    use gore_authoring::{
        AssetStoreIndex, FormatV2, GameGenerationAnchor, ProjectId, ProjectMeta,
        Revision2NpcParentClassInput, Revision3Entity, Revision3EntityKind, Revision3EntityPayload,
        Revision3NpcDraft, Revision3NpcDraftInput, Revision3OriginRef, Revision3TypedRef,
        SchemaRevisionV3, LOGICAL_NPC_CLONE_GENERATOR_ID, LOGICAL_NPC_CLONE_GENERATOR_VERSION,
    };
    use tempfile::TempDir;

    use super::*;

    const NPC_BYTE: u8 = 0x31;
    const MODULE_BYTE: u8 = 0x32;

    fn raw_request(payload: Value) -> String {
        json!({"command": COMMAND, "payload": payload}).to_string()
    }

    fn valid_shape() -> Value {
        json!({
            "expected_head_json": "{}",
            "npc_id": EntityId::from_bytes([NPC_BYTE; 16]).to_string(),
            "root": "C:/missing-store",
        })
    }

    fn digest(value: u8) -> Sha256Digest {
        Sha256Digest::from_bytes([value; 32])
    }

    fn content_seal(value: u8, byte_len: u64) -> ContentSeal {
        ContentSeal {
            byte_len,
            sha256: digest(value),
        }
    }

    fn target() -> GameGenerationAnchor {
        GameGenerationAnchor {
            executable: content_seal(1, 171_698_176),
        }
    }

    fn parent(value: u8, runtime_class: &str) -> Revision2NpcParentClassInput {
        Revision2NpcParentClassInput {
            generation: target(),
            source_seal: content_seal(value, 4_096),
            catalog_layer: "base-game.g1r.npc-parents.v1".to_owned(),
            canonical_selector: runtime_class.to_owned(),
            runtime_class: runtime_class.to_owned(),
        }
    }

    fn npc_project(revision: u64) -> ProjectRevision3 {
        let project_id = ProjectId::from_bytes([0x30; 16]);
        let npc_id = EntityId::from_bytes([NPC_BYTE; 16]);
        let module_id = EntityId::from_bytes([MODULE_BYTE; 16]);
        let owner = Revision3TypedRef::new(project_id, npc_id, Revision3EntityKind::NpcDraft);
        let draft = Revision3NpcDraft {
            generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
            generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
            input: Revision3NpcDraftInput {
                target: target(),
                module_namespace: "GoreMods.Npcs.NativeInspectionGuard".to_owned(),
                unique_name: "GORE_NATIVE_INSPECTION_GUARD".to_owned(),
                parent_character_definition: parent(
                    2,
                    "UCharacterDefinition_Human_OM_GRD_Asghan_263",
                ),
                parent_ai_agent_config: parent(3, "UAIAgentConfig_Human_OM_GRD_Asghan_263"),
                parent_spawn_definition: parent(4, "USpawnAIAgentDefinition_OM_GRD_Asghan_263"),
            },
            script_module: Revision3TypedRef::new(
                project_id,
                module_id,
                Revision3EntityKind::ScriptModule,
            ),
            greetings: Vec::new(),
        };
        let module = draft
            .regenerate_script_module(owner.clone())
            .expect("valid exact NPC fixture");
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id,
            revision,
            meta: ProjectMeta {
                name: "Native NPC inspection FFI".to_owned(),
                version: "1.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: target(),
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::from([
                (
                    npc_id,
                    Revision3Entity {
                        id: npc_id,
                        display_name: "Native Inspection Guard".to_owned(),
                        origin: Revision3OriginRef::New {
                            authored_runtime_id: draft.input.unique_name.clone(),
                        },
                        revision: 2,
                        payload: Revision3EntityPayload::NpcDraft(draft),
                    },
                ),
                (
                    module_id,
                    Revision3Entity {
                        id: module_id,
                        display_name: "Native Inspection Guard source".to_owned(),
                        origin: Revision3OriginRef::Generated {
                            generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
                            generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
                            owner,
                        },
                        revision: 3,
                        payload: Revision3EntityPayload::ScriptModule(module),
                    },
                ),
            ]),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn published_store(project: &ProjectRevision3) -> (TempDir, String, String) {
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let project_json = project.to_canonical_json().unwrap();
        let prepared = store.prepare_revision3_checkpoint(None, project).unwrap();
        fs::write(temp.path().join("gore-project.json"), &prepared.head_bytes).unwrap();
        (
            temp,
            String::from_utf8(prepared.head_bytes).unwrap(),
            project_json,
        )
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

    #[test]
    fn exact_raw_wire_rejects_duplicates_unknown_missing_and_wrong_types() {
        let parsed: InspectNpcWirePayload = parse_exact_wire(&raw_request(valid_shape())).unwrap();
        assert_eq!(
            parsed.npc_id,
            EntityId::from_bytes([NPC_BYTE; 16]).to_string()
        );

        let npc_id = EntityId::from_bytes([NPC_BYTE; 16]);
        let cases = [
            format!(
                "{{\"command\":\"{COMMAND}\",\"command\":\"{COMMAND}\",\"payload\":{{\"expected_head_json\":\"{{}}\",\"npc_id\":\"{npc_id}\",\"root\":\"r\"}}}}"
            ),
            format!(
                "{{\"command\":\"{COMMAND}\",\"payload\":{{\"expected_head_json\":\"{{}}\",\"npc_id\":\"{npc_id}\",\"npc_id\":\"{npc_id}\",\"root\":\"r\"}}}}"
            ),
            raw_request(json!({
                "expected_head_json": "{}", "npc_id": npc_id, "root": "r",
                "authority": true,
            })),
            raw_request(json!({
                "expected_head_json": "{}", "npc_id": npc_id, "root": "r",
                "game_root": "C:/forged-game",
            })),
            raw_request(json!({
                "expected_head_json": "{}", "npc_id": npc_id, "root": "r",
                "project_json": "{}", "source": "forged",
            })),
            raw_request(json!({"expected_head_json": "{}", "root": "r"})),
            raw_request(json!({
                "expected_head_json": {}, "npc_id": npc_id, "root": "r",
            })),
            raw_request(json!({
                "expected_head_json": "{}", "npc_id": npc_id, "root": 7,
            })),
            json!({"command": "wrong", "payload": valid_shape()}).to_string(),
        ];
        for input in cases {
            assert_eq!(
                inspect_revision3_npc_source_v1_raw(&input)["error"]["code"],
                "AUTHORING_REVISION3_NPC_INSPECTION_REQUEST_INVALID"
            );
        }
    }

    #[test]
    fn public_dispatch_preserves_duplicate_rejection_for_the_raw_route() {
        let npc_id = EntityId::from_bytes([NPC_BYTE; 16]);
        let duplicate = format!(
            "{{\"command\":\"{COMMAND}\",\"payload\":{{\"expected_head_json\":\"{{}}\",\"npc_id\":\"{npc_id}\",\"npc_id\":\"{npc_id}\",\"root\":\"r\"}}}}"
        );
        let response: Value = serde_json::from_str(&crate::execute_json(&duplicate)).unwrap();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_NPC_INSPECTION_REQUEST_INVALID"
        );
    }

    #[test]
    fn canonical_head_and_npc_identity_are_strict() {
        let project = npc_project(1);
        let (temp, head_json, _) = published_store(&project);
        assert!(parse_canonical_head(&head_json).is_ok());
        assert_eq!(
            parse_canonical_head(&format!(" {head_json}"))
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_NPC_INSPECTION_HEAD_INVALID"
        );
        let duplicate = head_json.replacen('{', "{\"store_format\":1,", 1);
        assert_eq!(
            parse_canonical_head(&duplicate).unwrap_err().code,
            "AUTHORING_REVISION3_NPC_INSPECTION_HEAD_INVALID"
        );
        assert!(parse_npc_id(&EntityId::from_bytes([NPC_BYTE; 16]).to_string()).is_ok());
        assert!(parse_npc_id("1").is_err());

        let response = inspect_revision3_npc_source_v1_raw(&raw_request(json!({
            "expected_head_json": head_json,
            "npc_id": "not-an-entity-id-not-an-entity-id",
            "root": temp.path(),
        })));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_NPC_INSPECTION_REQUEST_INVALID"
        );
    }

    #[test]
    fn exact_current_route_returns_closed_sealed_plan_without_mutating_store() {
        let project = npc_project(7);
        let npc_id = EntityId::from_bytes([NPC_BYTE; 16]);
        let (temp, head_json, project_json) = published_store(&project);
        let before = snapshot_regular_files(temp.path());

        let inspected = inspect_revision3_npc_source_v1_raw(&raw_request(json!({
            "expected_head_json": head_json,
            "npc_id": npc_id,
            "root": temp.path(),
        })));
        assert_eq!(inspected["ok"], true, "{inspected}");
        assert_eq!(inspected["outcome"], "inspection_only");
        assert_eq!(inspected["head_json"], head_json);
        assert_eq!(inspected["project_id"], project.project_id.to_string());
        assert_eq!(inspected["project_revision"], project.revision);
        assert_eq!(inspected["npc_id"], npc_id.to_string());
        assert_eq!(inspected["scope"], "source_readiness_inspection_only");
        assert_eq!(
            inspected["source_status"],
            "persisted_and_regenerated_exact"
        );
        assert_eq!(inspected["compiler_status"], "not_run");
        assert_eq!(inspected["build_status"], "blocked");
        assert_eq!(inspected["runtime_qualification"], "runtime_unqualified");
        assert_eq!(inspected["spawn_status"], "not_supported");
        assert_eq!(inspected["publication_status"], "not_supported");
        assert_eq!(
            inspected["project_seal"],
            serde_json::to_value(seal_bytes(project_json.as_bytes())).unwrap()
        );

        let plan_json = inspected["plan_json"].as_str().unwrap();
        let plan = Revision3NpcSourceInspectionPlanV1::from_json(plan_json).unwrap();
        assert_eq!(plan.npc().reference().id, npc_id);
        assert_eq!(plan.diagnostics().len(), 4);
        assert!(plan.diagnostics().iter().all(|item| item.blocks_build()));
        assert_eq!(
            inspected["plan_seal"],
            serde_json::to_value(plan.content_seal().unwrap()).unwrap()
        );
        assert_eq!(snapshot_regular_files(temp.path()), before);

        let encoded = inspected.to_string();
        assert!(!encoded.contains(temp.path().to_string_lossy().as_ref()));
        assert!(!encoded.contains(&project_json));
        for forbidden in [
            "game_root",
            "project_json",
            "catalog_json",
            "capability_json",
            "compile_status",
            "build_ready",
            "runtime_qualified",
            "spawn_supported",
            "publication_authority",
        ] {
            assert!(inspected.get(forbidden).is_none(), "leaked {forbidden}");
        }
    }

    #[test]
    fn stale_head_and_invalid_selection_fail_without_store_writes_or_path_leaks() {
        let project = npc_project(7);
        let (temp, head_json, _) = published_store(&project);
        let before = snapshot_regular_files(temp.path());

        let mut stale: WorkingHead = serde_json::from_str(&head_json).unwrap();
        stale.snapshot.sha256 = digest(0xee);
        let stale_response = inspect_revision3_npc_source_v1_raw(&raw_request(json!({
            "expected_head_json": serde_json::to_string(&stale).unwrap(),
            "npc_id": EntityId::from_bytes([NPC_BYTE; 16]),
            "root": temp.path(),
        })));
        assert_eq!(
            stale_response["error"]["code"],
            "AUTHORING_REVISION3_NPC_INSPECTION_HEAD_CONFLICT"
        );

        let missing = inspect_revision3_npc_source_v1_raw(&raw_request(json!({
            "expected_head_json": head_json,
            "npc_id": EntityId::from_bytes([0x55; 16]),
            "root": temp.path(),
        })));
        assert_eq!(
            missing["error"]["code"],
            "AUTHORING_REVISION3_NPC_INSPECTION_NPC_INVALID"
        );
        let module = inspect_revision3_npc_source_v1_raw(&raw_request(json!({
            "expected_head_json": head_json,
            "npc_id": EntityId::from_bytes([MODULE_BYTE; 16]),
            "root": temp.path(),
        })));
        assert_eq!(
            module["error"]["code"],
            "AUTHORING_REVISION3_NPC_INSPECTION_NPC_INVALID"
        );
        assert_eq!(snapshot_regular_files(temp.path()), before);
        for response in [stale_response, missing, module] {
            assert!(!response
                .to_string()
                .contains(temp.path().to_string_lossy().as_ref()));
        }
    }

    #[test]
    fn missing_store_root_is_sanitized_and_never_created() {
        let project = npc_project(1);
        let (temp, head_json, _) = published_store(&project);
        let missing = temp.path().join("absent-store");
        let before = snapshot_regular_files(temp.path());
        let response = inspect_revision3_npc_source_v1_raw(&raw_request(json!({
            "expected_head_json": head_json,
            "npc_id": EntityId::from_bytes([NPC_BYTE; 16]),
            "root": missing.to_string_lossy(),
        })));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_NPC_INSPECTION_STORE_ROOT_MISSING"
        );
        assert!(!missing.exists());
        assert_eq!(snapshot_regular_files(temp.path()), before);
        assert!(!response
            .to_string()
            .contains(missing.to_string_lossy().as_ref()));
    }

    #[test]
    fn paths_wire_response_and_error_messages_are_bounded() {
        let mut payload = valid_shape();
        payload["root"] = Value::String("x".repeat(MAX_PATH_BYTES + 1));
        assert_eq!(
            inspect_revision3_npc_source_v1_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_NPC_INSPECTION_REQUEST_INVALID"
        );
        let oversized = "x".repeat(MAX_WIRE_BYTES + 1);
        assert_eq!(
            inspect_revision3_npc_source_v1_raw(&oversized)["error"]["code"],
            "AUTHORING_REVISION3_NPC_INSPECTION_INPUT_LIMIT"
        );
        assert_eq!(
            signed_wire_u64(i64::MAX as u64 + 1).unwrap_err().code,
            "AUTHORING_REVISION3_NPC_INSPECTION_RESPONSE_LIMIT"
        );
        assert!(enforce_response_budget(json!({"ok": true})).is_ok());
        assert_eq!(
            enforce_response_budget(Value::String("x".repeat(MAX_RESPONSE_BYTES)))
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_NPC_INSPECTION_RESPONSE_LIMIT"
        );
        let truncated = Failure::new("TEST", "é".repeat(MAX_ERROR_MESSAGE_BYTES));
        assert!(truncated.message.len() <= MAX_ERROR_MESSAGE_BYTES);
        assert!(truncated.message.ends_with("..."));
    }

    #[test]
    fn inspection_error_mapping_separates_content_capacity_and_native_invariants() {
        let npc = EntityId::from_bytes([NPC_BYTE; 16]);
        assert_eq!(
            map_inspection_error(Revision3NpcInspectionErrorV1::MissingNpc(npc)).code,
            "AUTHORING_REVISION3_NPC_INSPECTION_NPC_INVALID"
        );
        assert_eq!(
            map_inspection_error(Revision3NpcInspectionErrorV1::ForeignGenerator { npc }).code,
            "AUTHORING_REVISION3_NPC_INSPECTION_PROJECT_INVALID"
        );
        assert_eq!(
            map_inspection_error(Revision3NpcInspectionErrorV1::PlanJsonTooLarge {
                actual: 5,
                limit: 4,
            })
            .code,
            "AUTHORING_REVISION3_NPC_INSPECTION_RESPONSE_LIMIT"
        );
        assert_eq!(
            map_inspection_error(Revision3NpcInspectionErrorV1::PlanInvariant(
                "forged authority".to_owned(),
            ))
            .code,
            "AUTHORING_REVISION3_NPC_INSPECTION_INVARIANT"
        );
    }
}
