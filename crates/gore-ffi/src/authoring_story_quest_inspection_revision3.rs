//! Exact-current, read-only source inspection for one managed revision-3 Quest.
//!
//! The client supplies only the selected working store, game installation, exact current head,
//! and Quest identity. Native code fully opens the current project, reconstructs the immutable
//! historical collision source referenced by the persisted Quest, rebuilds trusted game inputs,
//! and consumes an inspection-only capability to lower one sealed source plan. This route never
//! writes the Store, game, or saves and grants no compilation, build, runtime, publication, or
//! reusable collision authority.

use std::io;
use std::path::{Path, PathBuf};

use gore_authoring::{
    AssetVerification, ContentSeal, EntityId, ProjectRevision3, Sha256Digest, WorkingHead,
    WorkingProjectStore, WorkingStoreError, WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
};
use gore_story_build::revision3_quest::{
    prepare_revision3_quest_source_inspection_v3, QuestInspectionBuildStatus,
    QuestInspectionPublicationStatus, QuestInspectionRuntimeQualification, QuestInspectionScope,
    Revision3QuestInspectionError, Revision3QuestSourceInspectionPlanV3,
    MAX_REVISION3_QUEST_PLAN_JSON_BYTES,
};
use gore_story_catalog::{
    build_known_catalog_with_shipping_snapshot, CatalogError, GenerationInputLimits,
    StoryCatalogFile,
};
use gore_story_inventory::{
    build_base_game_inventory, Revision3QuestCollisionCapabilityErrorV2, StoryInventoryError,
    VerifiedRevision3QuestCollisionInspectionCapabilityV2, MAX_BINDS_CACHE_SOURCE_BYTES,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

use crate::authoring_story_inventory::{read_source_no_follow, SourceReadError};
use crate::err;

pub(super) const COMMAND: &str = "authoring_store_inspect_revision3_quest_source_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_QUEST_ID_BYTES: usize = 32;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
// The canonical plan is nested as a JSON string, so reserve a full second escaped copy.
const MAX_RESPONSE_BYTES: usize = MAX_REVISION3_QUEST_PLAN_JSON_BYTES * 2 + 1024 * 1024;
// JSON path strings can expand to six bytes per source byte. The other nested strings have a
// tighter two-byte escape bound. This remains far below the global transport ceiling.
const MAX_WIRE_BYTES: usize =
    MAX_PATH_BYTES * 12 + MAX_HEAD_JSON_BYTES * 2 + MAX_QUEST_ID_BYTES * 2 + 4 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectQuestWirePayload {
    expected_head_json: String,
    game_root: String,
    quest_id: String,
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

pub(super) fn inspect_revision3_quest_source_v1_raw(input: &str) -> Value {
    inspect_revision3_quest_source_v1_inner(input).unwrap_or_else(Failure::response)
}

fn inspect_revision3_quest_source_v1_inner(input: &str) -> Result<Value, Failure> {
    let payload: InspectQuestWirePayload = parse_exact_wire(input)?;
    validate_path(&payload.root)?;
    validate_path(&payload.game_root)?;
    let expected_head = parse_canonical_head(&payload.expected_head_json)?;
    let quest_id = parse_quest_id(&payload.quest_id)?;

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
            "AUTHORING_REVISION3_QUEST_INSPECTION_INVARIANT",
            "the fully opened revision-3 project could not be canonicalized",
        )
    })?;
    let project_seal = seal_bytes(canonical_project_json.as_bytes());

    let prepared =
        prepare_revision3_quest_source_inspection_v3(&store, &canonical_project_json, quest_id)
            .map_err(map_inspection_error)?;
    let inspection_source = prepared
        .prepare_collision_inspection_source(&store)
        .map_err(map_inspection_error)?;

    let game_root = PathBuf::from(&payload.game_root);
    let (catalog, shipping, binds) = build_fresh_game_inputs(&game_root)?;
    let inventory =
        build_base_game_inventory(&catalog, &shipping, &binds).map_err(map_inventory_error)?;
    let capability = VerifiedRevision3QuestCollisionInspectionCapabilityV2::bind(
        inventory,
        &catalog,
        inspection_source,
    )
    .map_err(map_capability_error)?;
    let plan = prepared.lower(capability).map_err(map_inspection_error)?;
    validate_closed_plan(&plan, &before.project, &project_seal, quest_id)?;
    let plan_json = plan.to_canonical_json().map_err(map_inspection_error)?;
    let reopened = Revision3QuestSourceInspectionPlanV3::from_json(&plan_json)
        .map_err(map_inspection_error)?;
    if reopened != plan {
        return Err(inspection_failed());
    }
    let plan_seal = plan.content_seal().map_err(map_inspection_error)?;

    // Close the mutable native-input window after lowering has consumed the one-shot capability.
    revalidate_game_inputs(&catalog, &game_root, &shipping)?;
    // A second full open proves that neither the fixed head, canonical project, nor referenced
    // Store assets drifted during historical reconstruction and source lowering.
    let after = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if after.head != expected_head || after.project != before.project {
        return Err(head_conflict());
    }
    let head_json = serde_json::to_string(&after.head).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_INVARIANT",
            "the exact revision-3 working head could not be serialized",
        )
    })?;
    if head_json != payload.expected_head_json {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_HEAD_INVALID",
            "expected_head_json is not in exact canonical form",
        ));
    }
    // Recheck external game inputs once more after the closing Store read. No source evidence is
    // returned if the installation changed anywhere across the inspection window.
    revalidate_game_inputs(&catalog, &game_root, &shipping)?;

    enforce_response_budget(json!({
        "ok": true,
        "outcome": "inspection_only",
        "head_json": head_json,
        "project_id": after.project.project_id.to_string(),
        "project_revision": after.project.revision,
        "project_seal": project_seal,
        "quest_id": quest_id.to_string(),
        "plan_json": plan_json,
        "plan_seal": plan_seal,
        "scope": "source_inspection_only",
        "build_status": "blocked",
        "runtime_qualification": "runtime_unqualified",
        "publication_status": "not_supported",
    }))
}

fn parse_exact_wire<P: DeserializeOwned>(input: &str) -> Result<P, Failure> {
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_LIMIT",
            format!("Quest inspection request exceeds the {MAX_WIRE_BYTES}-byte wire limit"),
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
            "AUTHORING_REVISION3_QUEST_INSPECTION_HEAD_INVALID",
            "expected_head_json is empty or exceeds its bounded transport limit",
        ));
    }
    let head: WorkingHead = serde_json::from_str(input).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_HEAD_INVALID",
            "expected_head_json is not one closed working head",
        )
    })?;
    let canonical = serde_json::to_string(&head).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_INVARIANT",
            "the exact revision-3 working head could not be serialized",
        )
    })?;
    if canonical != input {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_HEAD_INVALID",
            "expected_head_json is not duplicate-free canonical JSON",
        ));
    }
    Ok(head)
}

fn parse_quest_id(input: &str) -> Result<EntityId, Failure> {
    if input.len() != MAX_QUEST_ID_BYTES {
        return Err(invalid_request());
    }
    input.parse().map_err(|_| invalid_request())
}

fn build_fresh_game_inputs(
    game_root: &Path,
) -> Result<(StoryCatalogFile, Vec<u8>, Vec<u8>), Failure> {
    let g1r = resolve_g1r_root(game_root);
    let executable = g1r
        .join("Binaries")
        .join("Win64")
        .join("G1R-Win64-Shipping.exe");
    let binds_path = g1r.join("Script").join("Binds.Cache");
    let shipping = gore_mod::pristine_script_cache(game_root).map_err(map_pristine_error)?;
    let catalog = build_known_catalog_with_shipping_snapshot(
        &executable,
        &shipping,
        &binds_path,
        GenerationInputLimits::default(),
    )
    .map_err(map_catalog_error)?;
    catalog
        .revalidate_generation_inputs()
        .map_err(map_catalog_error)?;
    let binds = read_source_no_follow(&binds_path, MAX_BINDS_CACHE_SOURCE_BYTES as u64)
        .map_err(map_source_read_error)?;
    if binds.len() as u64 != catalog.generation().binds_cache.byte_len
        || Sha256::digest(&binds).as_slice() != catalog.generation().binds_cache.sha256.as_bytes()
    {
        return Err(input_changed());
    }
    catalog
        .revalidate_generation_inputs()
        .map_err(map_catalog_error)?;
    Ok((catalog, shipping, binds))
}

fn resolve_g1r_root(game_root: &Path) -> PathBuf {
    if game_root.file_name().is_some_and(is_g1r_component) {
        game_root.to_path_buf()
    } else {
        game_root.join("G1R")
    }
}

fn is_g1r_component(value: &std::ffi::OsStr) -> bool {
    value.as_encoded_bytes().eq_ignore_ascii_case(b"G1R")
}

fn revalidate_game_inputs(
    catalog: &StoryCatalogFile,
    game_root: &Path,
    expected_shipping: &[u8],
) -> Result<(), Failure> {
    catalog
        .revalidate_generation_inputs()
        .map_err(map_catalog_error)?;
    let current = gore_mod::pristine_script_cache(game_root).map_err(map_pristine_error)?;
    if current.len() != expected_shipping.len()
        || Sha256::digest(&current).as_slice() != Sha256::digest(expected_shipping).as_slice()
    {
        return Err(input_changed());
    }
    catalog
        .revalidate_generation_inputs()
        .map_err(map_catalog_error)
}

fn validate_closed_plan(
    plan: &Revision3QuestSourceInspectionPlanV3,
    project: &ProjectRevision3,
    project_seal: &ContentSeal,
    quest_id: EntityId,
) -> Result<(), Failure> {
    if plan.schema_revision() != 3
        || plan.scope != QuestInspectionScope::SourceInspectionOnly
        || plan.build_status != QuestInspectionBuildStatus::Blocked
        || plan.runtime_qualification != QuestInspectionRuntimeQualification::RuntimeUnqualified
        || plan.publication_status != QuestInspectionPublicationStatus::NotSupported
        || plan.provenance.project_id != project.project_id
        || plan.provenance.project_revision != project.revision
        || plan.provenance.target_executable != project.target.executable
        || &plan.provenance.canonical_project != project_seal
        || plan.module.quest.project_id != project.project_id
        || plan.module.quest.id != quest_id
    {
        return Err(inspection_failed());
    }
    signed_wire_u64(plan.provenance.collision_prior_quest_count)?;
    for seal in [
        &plan.provenance.target_executable,
        &plan.provenance.canonical_project,
        &plan.provenance.collision_basis_head.snapshot,
        &plan.provenance.collision_basis_project,
        &plan.provenance.collision_nonquest_project,
        &plan.provenance.collision_prior_quest_evidence,
        &plan.provenance.collision_artifact,
        &plan.provenance.collision_source,
        &plan.module.draft_input,
        &plan.module.persisted_source,
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
            "AUTHORING_REVISION3_QUEST_INSPECTION_RESPONSE_LIMIT",
            "Quest inspection contains an integer outside the signed wire range",
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
            "AUTHORING_REVISION3_QUEST_INSPECTION_INVARIANT",
            "Quest inspection response could not be serialized",
        )
    })?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_RESPONSE_LIMIT",
            "Quest inspection response exceeds its bounded transport budget",
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
        "AUTHORING_REVISION3_QUEST_INSPECTION_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and exactly expected_head_json, game_root, quest_id, and root",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_INSPECTION_HEAD_CONFLICT",
        "the published revision-3 head changed or differs from the caller's exact head",
    )
}

fn input_changed() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_CHANGED",
        "the native game generation changed during Quest inspection",
    )
}

fn inspection_failed() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_INSPECTION_FAILED",
        "the exact read-only Quest source inspection could not be produced",
    )
}

fn map_pristine_error(error: gore_mod::ModError) -> Failure {
    let message = error.to_string();
    if message.contains("RECOVERY_REQUIRED") {
        return Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_RECOVERY_REQUIRED",
            "an interrupted deployment must be recovered before Quest inspection",
        );
    }
    if message.contains("exceeds the") || message.contains("too large") {
        return Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_LIMIT",
            "the pristine Shipping cache exceeds its bounded input limit",
        );
    }
    if message.contains("not a regular non-link file") {
        return Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_UNSAFE",
            "the pristine Shipping cache is not a safe regular file",
        );
    }
    Failure::new(
        "AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_UNAVAILABLE",
        "the pristine Shipping cache could not be selected safely",
    )
}

fn map_catalog_error(error: CatalogError) -> Failure {
    match error {
        CatalogError::InvalidLimits(_) | CatalogError::LimitExceeded { .. } => Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_LIMIT",
            "a native game-generation input exceeds its bounded resource limit",
        ),
        CatalogError::UnsafeInput(_) | CatalogError::OutputAliasesInput { .. } => Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_UNSAFE",
            "a native game-generation input is unsafe",
        ),
        CatalogError::IdentityChanged(_) | CatalogError::SourceChanged { .. } => input_changed(),
        CatalogError::UnsupportedGeneration { .. } => Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_UNSUPPORTED_GENERATION",
            "the selected game does not match the supported pinned generation",
        ),
        CatalogError::Io { source, .. } if source.kind() == io::ErrorKind::NotFound => {
            Failure::new(
                "AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_MISSING",
                "a required native game-generation input does not exist",
            )
        }
        _ => Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_UNAVAILABLE",
            "the native game generation could not be verified safely",
        ),
    }
}

fn map_source_read_error(error: SourceReadError) -> Failure {
    match error {
        SourceReadError::Missing => Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_MISSING",
            "a required native game-generation input does not exist",
        ),
        SourceReadError::Unsafe => Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_UNSAFE",
            "a native game-generation input is unsafe",
        ),
        SourceReadError::Limit => Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_LIMIT",
            "a native game-generation input exceeds its bounded resource limit",
        ),
        SourceReadError::Changed => input_changed(),
        SourceReadError::Io => Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_UNAVAILABLE",
            "a native game-generation input could not be read safely",
        ),
    }
}

fn map_inventory_error(error: StoryInventoryError) -> Failure {
    match error {
        StoryInventoryError::LimitExceeded { .. } | StoryInventoryError::SourcePairTooLarge => {
            Failure::new(
                "AUTHORING_REVISION3_QUEST_INSPECTION_COLLISION_LIMIT",
                "the trusted collision inventory exceeds its bounded resource limit",
            )
        }
        StoryInventoryError::UnsupportedGeneration => Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_UNSUPPORTED_GENERATION",
            "the selected game does not match the supported pinned generation",
        ),
        StoryInventoryError::SourceLengthMismatch { .. }
        | StoryInventoryError::SourceDigestMismatch { .. }
        | StoryInventoryError::SourcePairSealMismatch
        | StoryInventoryError::RecollectedInventoryMismatch => input_changed(),
        _ => Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_INVENTORY_FAILED",
            "the trusted base-game collision inventory could not be rebuilt",
        ),
    }
}

fn map_capability_error(error: Revision3QuestCollisionCapabilityErrorV2) -> Failure {
    match error {
        Revision3QuestCollisionCapabilityErrorV2::TargetMismatch => Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_PROJECT_TARGET_MISMATCH",
            "the persisted Quest basis does not target the trusted game generation",
        ),
        Revision3QuestCollisionCapabilityErrorV2::Limit { .. } => Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_COLLISION_LIMIT",
            "the combined collision evidence exceeds its bounded resource limit",
        ),
        _ => inspection_failed(),
    }
}

fn map_inspection_error(error: Revision3QuestInspectionError) -> Failure {
    use Revision3QuestInspectionError as E;
    match error {
        E::ProjectJsonTooLarge { .. } | E::PlanJsonTooLarge { .. } => Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_RESPONSE_LIMIT",
            "Quest inspection exceeds its bounded resource limit",
        ),
        E::MissingQuest(_) | E::NotAQuest(_) => Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_QUEST_INVALID",
            "quest_id does not identify one managed revision-3 Quest",
        ),
        E::ArtifactUnavailable { source, .. } | E::BasisUnavailable { source, .. } => {
            map_store_error(source)
        }
        E::HistoricalInspectionSource(source) => match source {
            gore_authoring::Revision3QuestCollisionSourceErrorV2::Store(source) => {
                map_store_error(source)
            }
            gore_authoring::Revision3QuestCollisionSourceErrorV2::CurrentSnapshotDrift => {
                head_conflict()
            }
            gore_authoring::Revision3QuestCollisionSourceErrorV2::Limit { .. }
            | gore_authoring::Revision3QuestCollisionSourceErrorV2::TooManyPriorQuests { .. } => {
                Failure::new(
                    "AUTHORING_REVISION3_QUEST_INSPECTION_COLLISION_LIMIT",
                    "the historical Quest collision source exceeds its bounded resource limit",
                )
            }
            _ => inspection_failed(),
        },
        E::InvalidProjectDocument(_)
        | E::NonCanonicalProjectJson
        | E::Revision3Required
        | E::ForeignProject { .. }
        | E::ForeignGeneration { .. }
        | E::ForeignGenerator { .. }
        | E::OwnerMismatch { .. } => Failure::new(
            "AUTHORING_REVISION3_QUEST_INSPECTION_PROJECT_INVALID",
            "the fully opened current project is not a valid managed Quest inspection source",
        ),
        _ => inspection_failed(),
    }
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match &error {
        WorkingStoreError::InvalidLimits(_) => {
            "AUTHORING_REVISION3_QUEST_INSPECTION_STORE_LIMITS_INVALID"
        }
        WorkingStoreError::MissingRoot(_) => {
            "AUTHORING_REVISION3_QUEST_INSPECTION_STORE_ROOT_MISSING"
        }
        WorkingStoreError::UnsafePath { .. } => {
            "AUTHORING_REVISION3_QUEST_INSPECTION_STORE_PATH_UNSAFE"
        }
        WorkingStoreError::LimitExceeded { .. } => {
            "AUTHORING_REVISION3_QUEST_INSPECTION_STORE_LIMIT"
        }
        WorkingStoreError::HeadConflict { .. } => {
            "AUTHORING_REVISION3_QUEST_INSPECTION_HEAD_CONFLICT"
        }
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_QUEST_INSPECTION_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => {
            "AUTHORING_REVISION3_QUEST_INSPECTION_STORE_OBJECT_MISSING"
        }
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_QUEST_INSPECTION_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => {
            "AUTHORING_REVISION3_QUEST_INSPECTION_STORE_COLLISION"
        }
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_QUEST_INSPECTION_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_QUEST_INSPECTION_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_QUEST_INSPECTION_STORE_IO"
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
    use std::collections::BTreeMap;
    use std::fs;

    use gore_authoring::{
        FormatV2, GameGenerationAnchor, ProjectId, ProjectMeta, SchemaRevisionV3,
    };
    use gore_story_catalog::known_generation_v1;
    use gore_story_inventory::Revision3QuestDraftInsertRequestV3;
    use tempfile::TempDir;

    use super::*;

    fn raw_request(payload: Value) -> String {
        json!({"command": COMMAND, "payload": payload}).to_string()
    }

    fn valid_shape() -> Value {
        json!({
            "expected_head_json": "{}",
            "game_root": "C:/missing-game",
            "quest_id": "01010101010101010101010101010101",
            "root": "C:/missing-store",
        })
    }

    fn empty_project(revision: u64) -> ProjectRevision3 {
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([1; 16]),
            revision,
            meta: ProjectMeta {
                name: "Quest inspection FFI".to_owned(),
                version: "0.1.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: GameGenerationAnchor {
                executable: ContentSeal {
                    byte_len: 4,
                    sha256: Sha256Digest::from_bytes([2; 32]),
                },
            },
            authoring_locales: Default::default(),
            entities: BTreeMap::new(),
            asset_store: Default::default(),
        }
    }

    fn published_store(revision: u64) -> (TempDir, String) {
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let prepared = store
            .prepare_revision3_checkpoint(None, &empty_project(revision))
            .unwrap();
        fs::write(temp.path().join("gore-project.json"), &prepared.head_bytes).unwrap();
        (temp, String::from_utf8(prepared.head_bytes).unwrap())
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
        let parsed: InspectQuestWirePayload =
            parse_exact_wire(&raw_request(valid_shape())).unwrap();
        assert_eq!(parsed.quest_id, "01010101010101010101010101010101");

        let cases = [
            format!(
                "{{\"command\":\"{COMMAND}\",\"command\":\"{COMMAND}\",\"payload\":{{\"expected_head_json\":\"{{}}\",\"game_root\":\"g\",\"quest_id\":\"01010101010101010101010101010101\",\"root\":\"r\"}}}}"
            ),
            format!(
                "{{\"command\":\"{COMMAND}\",\"payload\":{{\"expected_head_json\":\"{{}}\",\"game_root\":\"g\",\"game_root\":\"forged\",\"quest_id\":\"01010101010101010101010101010101\",\"root\":\"r\"}}}}"
            ),
            raw_request(json!({
                "expected_head_json": "{}", "game_root": "g",
                "quest_id": "01010101010101010101010101010101", "root": "r",
                "capability": true,
            })),
            raw_request(json!({
                "expected_head_json": "{}", "game_root": "g", "root": "r",
            })),
            raw_request(json!({
                "expected_head_json": {}, "game_root": "g",
                "quest_id": "01010101010101010101010101010101", "root": "r",
            })),
        ];
        for input in cases {
            assert_eq!(
                inspect_revision3_quest_source_v1_raw(&input)["error"]["code"],
                "AUTHORING_REVISION3_QUEST_INSPECTION_REQUEST_INVALID"
            );
        }
    }

    #[test]
    fn public_dispatch_preserves_duplicate_rejection_for_the_raw_route() {
        let duplicate = format!(
            "{{\"command\":\"{COMMAND}\",\"payload\":{{\"expected_head_json\":\"{{}}\",\"game_root\":\"g\",\"quest_id\":\"01010101010101010101010101010101\",\"quest_id\":\"02020202020202020202020202020202\",\"root\":\"r\"}}}}"
        );
        let response: Value = serde_json::from_str(&crate::execute_json(&duplicate)).unwrap();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_QUEST_INSPECTION_REQUEST_INVALID"
        );
    }

    #[test]
    fn canonical_head_and_quest_identity_are_strict() {
        let (temp, head_json) = published_store(1);
        assert!(parse_canonical_head(&head_json).is_ok());
        assert_eq!(
            parse_canonical_head(&format!(" {head_json}"))
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_QUEST_INSPECTION_HEAD_INVALID"
        );
        let duplicate = head_json.replacen("{", "{\"store_format\":1,", 1);
        assert_eq!(
            parse_canonical_head(&duplicate).unwrap_err().code,
            "AUTHORING_REVISION3_QUEST_INSPECTION_HEAD_INVALID"
        );
        assert!(parse_quest_id("01010101010101010101010101010101").is_ok());
        assert!(parse_quest_id("1").is_err());

        let response = inspect_revision3_quest_source_v1_raw(&raw_request(json!({
            "expected_head_json": head_json,
            "game_root": temp.path().join("missing-game"),
            "quest_id": "not-an-entity-id-not-an-entity-id",
            "root": temp.path(),
        })));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_QUEST_INSPECTION_REQUEST_INVALID"
        );
    }

    #[test]
    fn stale_head_and_missing_quest_fail_before_game_access_without_store_writes() {
        let (temp, head_json) = published_store(7);
        let before = snapshot_regular_files(temp.path());

        let mut stale: WorkingHead = serde_json::from_str(&head_json).unwrap();
        stale.snapshot.sha256 = Sha256Digest::from_bytes([9; 32]);
        let stale_response = inspect_revision3_quest_source_v1_raw(&raw_request(json!({
            "expected_head_json": serde_json::to_string(&stale).unwrap(),
            "game_root": temp.path().join("missing-game"),
            "quest_id": "01010101010101010101010101010101",
            "root": temp.path(),
        })));
        assert_eq!(
            stale_response["error"]["code"],
            "AUTHORING_REVISION3_QUEST_INSPECTION_HEAD_CONFLICT"
        );

        let missing_quest = inspect_revision3_quest_source_v1_raw(&raw_request(json!({
            "expected_head_json": head_json,
            "game_root": temp.path().join("missing-game"),
            "quest_id": "01010101010101010101010101010101",
            "root": temp.path(),
        })));
        assert_eq!(
            missing_quest["error"]["code"],
            "AUTHORING_REVISION3_QUEST_INSPECTION_QUEST_INVALID"
        );
        assert_eq!(snapshot_regular_files(temp.path()), before);
        let encoded = missing_quest.to_string();
        assert!(!encoded.contains(temp.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn paths_wire_and_response_values_are_bounded() {
        let mut payload = valid_shape();
        payload["root"] = Value::String("x".repeat(MAX_PATH_BYTES + 1));
        assert_eq!(
            inspect_revision3_quest_source_v1_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_QUEST_INSPECTION_REQUEST_INVALID"
        );
        assert_eq!(
            signed_wire_u64(i64::MAX as u64 + 1).unwrap_err().code,
            "AUTHORING_REVISION3_QUEST_INSPECTION_RESPONSE_LIMIT"
        );
        assert!(enforce_response_budget(json!({"ok": true})).is_ok());
    }

    #[test]
    fn g1r_root_resolver_is_case_insensitive_and_never_double_appends() {
        let install = Path::new("C:/Games/Gothic");
        assert_eq!(resolve_g1r_root(install), install.join("G1R"));
        for component in ["G1R", "g1r", "G1r", "g1R"] {
            let direct = install.join(component);
            assert_eq!(resolve_g1r_root(&direct), direct);
        }
    }

    #[test]
    #[ignore = "requires GORE_STORY_GAME_ROOT pointing at the pinned supported game generation"]
    fn live_native_route_inspects_a_published_quest_without_mutating_store_or_game() {
        let game_root = std::env::var("GORE_STORY_GAME_ROOT")
            .expect("set GORE_STORY_GAME_ROOT to run the live Quest inspection FFI test");
        let generation = known_generation_v1();
        let project: ProjectRevision3 = serde_json::from_value(json!({
            "format": 2,
            "schema_revision": 3,
            "project_id": "93939393939393939393939393939393",
            "revision": 0,
            "meta": {"name": "Live inspection FFI", "version": "1.0.0", "author": "tests"},
            "target": {"executable": generation.executable},
            "authoring_locales": [],
            "entities": {},
            "asset_store": {"assets": {}}
        }))
        .unwrap();
        let project_json = project.to_canonical_json().unwrap();
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let published = store.prepare_revision3_checkpoint(None, &project).unwrap();
        fs::write(temp.path().join("gore-project.json"), &published.head_bytes).unwrap();

        let quest_id = EntityId::from_bytes([0x80; 16]);
        let request: Revision3QuestDraftInsertRequestV3 = serde_json::from_value(json!({
            "expected_head": published.head,
            "expected_project_id": project.project_id,
            "expected_revision": project.revision,
            "quest_id": quest_id,
            "script_module_id": EntityId::from_bytes([0x81; 16]),
            "display_name": "Native inspection Quest",
            "intent": {
                "module_namespace": "GoreMods.Quests.NativeInspectionQuest",
                "technical_id": "GORE_NATIVE_INSPECTION_QUEST",
                "text_helper": "GoreNativeInspectionQuestText",
                "parent_catalog_id": "g1r:quest-parent:swampcamp_scchapter2",
                "giver_catalog_id": "g1r:npc:om_grd_asghan_263",
                "title": "Inspect native source",
                "description": "Exercise the exact read-only FFI path.",
                "objective_title": "Inspect the source",
                "additional_objective_titles": ["Verify provenance"],
            }
        }))
        .unwrap();
        let draft = crate::authoring_story_quest_revision3::prepare_revision3_quest_draft_v3_raw(
            &json!({
                "command": crate::authoring_story_quest_revision3::COMMAND,
                "payload": {
                    "current_project_json": project_json,
                    "game_root": game_root,
                    "quest_request_json": request.to_canonical_json().unwrap(),
                    "root": temp.path(),
                }
            })
            .to_string(),
        );
        assert_eq!(draft["ok"], true, "{draft}");
        let quest_head_json = draft["head_json"].as_str().unwrap();
        let quest_project_json = draft["project_json"].as_str().unwrap();
        fs::write(temp.path().join("gore-project.json"), quest_head_json).unwrap();
        let store_before_inspection = snapshot_regular_files(temp.path());

        let inspected = inspect_revision3_quest_source_v1_raw(&raw_request(json!({
            "expected_head_json": quest_head_json,
            "game_root": std::env::var("GORE_STORY_GAME_ROOT").unwrap(),
            "quest_id": quest_id,
            "root": temp.path(),
        })));
        assert_eq!(inspected["ok"], true, "{inspected}");
        assert_eq!(inspected["outcome"], "inspection_only");
        assert_eq!(inspected["head_json"], quest_head_json);
        assert_eq!(inspected["project_id"], project.project_id.to_string());
        assert_eq!(inspected["project_revision"], 1);
        assert_eq!(inspected["quest_id"], quest_id.to_string());
        assert_eq!(inspected["scope"], "source_inspection_only");
        assert_eq!(inspected["build_status"], "blocked");
        assert_eq!(inspected["runtime_qualification"], "runtime_unqualified");
        assert_eq!(inspected["publication_status"], "not_supported");
        assert_eq!(
            inspected["project_seal"],
            serde_json::to_value(seal_bytes(quest_project_json.as_bytes())).unwrap()
        );

        let plan_json = inspected["plan_json"].as_str().unwrap();
        let plan = Revision3QuestSourceInspectionPlanV3::from_json(plan_json).unwrap();
        assert_eq!(plan.schema_revision(), 3);
        assert_eq!(plan.module.quest.id, quest_id);
        assert_eq!(
            inspected["plan_seal"],
            serde_json::to_value(plan.content_seal().unwrap()).unwrap()
        );
        assert_eq!(snapshot_regular_files(temp.path()), store_before_inspection);
        let encoded = inspected.to_string();
        assert!(!encoded.contains(temp.path().to_string_lossy().as_ref()));
        assert!(!encoded.contains(&std::env::var("GORE_STORY_GAME_ROOT").unwrap()));
        for forbidden in [
            "artifact_json",
            "capability_json",
            "catalog_json",
            "project_json",
            "publication_authority",
            "runtime_qualified",
        ] {
            assert!(inspected.get(forbidden).is_none());
        }
    }
}
