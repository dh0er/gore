//! Native, prepare-only orchestration for one revision-3 Quest Draft transaction.
//!
//! The client supplies only exact project/request transports plus a selected working-store and
//! game root. Native code rebuilds the trusted catalog and base-game collision inventory, binds
//! them to the exact published revision-3 source, consumes the linear transaction capability,
//! imports the structural artifact, and prepares a fully reopened immutable checkpoint. The
//! fixed `gore-project.json` head is never replaced here.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use gore_authoring::{
    AssetVerification, Revision3QuestCollisionSourceErrorV2, WorkingHead, WorkingProjectStore,
    WorkingStoreError, WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
};
use gore_story_catalog::{
    build_known_catalog_with_shipping_snapshot, CatalogError, GenerationInputLimits,
    StoryCatalogFile,
};
use gore_story_inventory::{
    apply_revision3_quest_draft_transaction_v3, build_base_game_inventory,
    prepare_revision3_quest_draft_persistence_v3, QuestCollisionCapabilityArtifactErrorV2,
    Revision3QuestCollisionCapabilityErrorV2, Revision3QuestDraftInsertErrorV3,
    Revision3QuestDraftPersistenceErrorV3, Revision3QuestDraftPersistenceValidationErrorV3,
    Revision3QuestDraftProjectTransportErrorV3, StoryInventoryError,
    VerifiedRevision3QuestCollisionCapabilityV2, MAX_BINDS_CACHE_SOURCE_BYTES,
    MAX_REVISION3_QUEST_DRAFT_REQUEST_JSON_BYTES_V3,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

use crate::authoring_story_inventory::{read_source_no_follow, SourceReadError};
use crate::err;

pub(super) const COMMAND: &str = "authoring_store_prepare_revision3_quest_draft_v3";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
// The candidate increments this once and every Studio wire must remain signed-64-bit safe.
const MAX_BASIS_REVISION: u64 = i64::MAX as u64 - 1;
// Nested JSON strings can use two bytes on the outer wire for every source byte. Paths can use
// six-byte JSON escapes. Keep this route-local parser below the global 64 MiB transport ceiling.
const MAX_WIRE_BYTES: usize = MAX_PROJECT_JSON_BYTES * 2
    + MAX_REVISION3_QUEST_DRAFT_REQUEST_JSON_BYTES_V3 * 2
    + MAX_PATH_BYTES * 12
    + 8 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PrepareQuestWirePayload {
    current_project_json: String,
    game_root: String,
    quest_request_json: String,
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

pub(super) fn prepare_revision3_quest_draft_v3_raw(input: &str) -> Value {
    prepare_revision3_quest_draft_v3_inner(input).unwrap_or_else(Failure::response)
}

fn prepare_revision3_quest_draft_v3_inner(input: &str) -> Result<Value, Failure> {
    let payload: PrepareQuestWirePayload = parse_exact_wire(input)?;
    validate_payload(&payload)?;

    let root = Path::new(&payload.root);
    let store =
        WorkingProjectStore::open_existing(root, ffi_store_limits()).map_err(map_store_error)?;
    let basis = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    validate_basis_revision(basis.project.revision)?;
    let basis_source = store
        .prepare_current_revision3_quest_collision_source_v2(&basis.head)
        .map_err(map_basis_source_error)?;

    let game_root = PathBuf::from(&payload.game_root);
    let (catalog, shipping, binds) = build_fresh_game_inputs(&game_root)?;
    ensure_store_is_outside_game(&store, &game_root)?;
    let inventory =
        build_base_game_inventory(&catalog, &shipping, &binds).map_err(map_inventory_error)?;
    let capability =
        VerifiedRevision3QuestCollisionCapabilityV2::bind(inventory, &catalog, basis_source)
            .map_err(map_capability_error)?;
    let prepared_artifact = capability.prepare_artifact().map_err(map_artifact_error)?;

    let outcome = apply_revision3_quest_draft_transaction_v3(
        prepared_artifact,
        &payload.current_project_json,
        &payload.quest_request_json,
    )
    .map_err(map_transaction_error)?;

    // Close the native-input window before the first immutable store object may be installed.
    revalidate_game_inputs(&catalog, &game_root, &shipping)?;
    // Re-resolve both roots at the actual write boundary. A path swap after the earlier guard may
    // at worst consume CPU; it must never redirect artifact/checkpoint CAS writes into the game
    // installation.
    ensure_store_is_outside_game(&store, &game_root)?;
    let prepared = prepare_revision3_quest_draft_persistence_v3(&store, outcome)
        .map_err(map_persistence_error)?;

    let candidate_head_json =
        String::from_utf8(prepared.checkpoint.head_bytes.clone()).map_err(|_| {
            Failure::new(
                "AUTHORING_REVISION3_QUEST_INVARIANT",
                "prepared revision-3 Quest head is not UTF-8 JSON",
            )
        })?;
    if candidate_head_json.is_empty() || candidate_head_json.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_RESPONSE_LIMIT",
            "prepared revision-3 Quest head exceeds its bounded transport limit",
        ));
    }
    let basis_head_json = canonical_head_json(&prepared.basis_head)?;
    let response = json!({
        "ok": true,
        "outcome": "prepared_unpublished",
        "basis_head_json": basis_head_json,
        "head_json": candidate_head_json,
        "project_json": prepared.canonical_project_json,
        "revision": prepared.project.revision,
        "quest_id": prepared.quest_id.to_string(),
        "script_module_id": prepared.script_module_id.to_string(),
        "artifact_deduplicated": prepared.imported_artifact.deduplicated,
        "build_status": "blocked",
        "runtime_status": "runtime_unqualified",
        "artifact_authority": "not_granted",
        "source_inspection": "fresh_capability_required",
        "publication_status": "not_supported",
    });
    enforce_response_budget(&response)?;

    // A late game-source race can leave only verified immutable CAS orphans. The fixed project
    // head still has not been touched, and no response is returned for stale provenance.
    revalidate_game_inputs(&catalog, &game_root, &shipping)?;
    Ok(response)
}

fn parse_exact_wire<P: DeserializeOwned>(input: &str) -> Result<P, Failure> {
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_INPUT_LIMIT",
            format!("revision-3 Quest request exceeds the {MAX_WIRE_BYTES}-byte wire limit"),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_payload(payload: &PrepareQuestWirePayload) -> Result<(), Failure> {
    validate_path(&payload.root)?;
    validate_path(&payload.game_root)?;
    if payload.current_project_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_PROJECT_LIMIT",
            format!("current_project_json exceeds the {MAX_PROJECT_JSON_BYTES}-byte project limit"),
        ));
    }
    if payload.quest_request_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.quest_request_json.len() > MAX_REVISION3_QUEST_DRAFT_REQUEST_JSON_BYTES_V3 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_REQUEST_LIMIT",
            format!(
                "quest_request_json exceeds the {MAX_REVISION3_QUEST_DRAFT_REQUEST_JSON_BYTES_V3}-byte request limit"
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
    // `build_base_game_inventory` will also bind these bytes to the catalog seal. Checking here
    // keeps a changed Binds path out of the larger collision parser.
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

fn ensure_store_is_outside_game(
    store: &WorkingProjectStore,
    game_root: &Path,
) -> Result<(), Failure> {
    let store_root = fs::canonicalize(store.root()).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_STORE_PATH_UNSAFE",
            "the revision-3 working-store root could not be resolved safely",
        )
    })?;
    let semantic_install_root = semantic_install_root(game_root);
    let semantic_install_root = fs::canonicalize(&semantic_install_root).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_INPUT_UNAVAILABLE",
            "the selected game installation root could not be resolved safely",
        )
    })?;
    if store_root.starts_with(&semantic_install_root)
        || semantic_install_root.starts_with(&store_root)
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_STORE_GAME_ALIAS",
            "the working-store root and selected game installation must be disjoint",
        ));
    }
    Ok(())
}

fn semantic_install_root(game_root: &Path) -> PathBuf {
    if game_root.file_name().is_some_and(is_g1r_component) {
        let parent = game_root
            .parent()
            .filter(|value| !value.as_os_str().is_empty());
        parent.unwrap_or_else(|| Path::new(".")).to_path_buf()
    } else {
        game_root.to_path_buf()
    }
}

fn is_g1r_component(value: &std::ffi::OsStr) -> bool {
    value.as_encoded_bytes().eq_ignore_ascii_case(b"G1R")
}

fn validate_basis_revision(revision: u64) -> Result<(), Failure> {
    if revision > MAX_BASIS_REVISION {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_REVISION_LIMIT",
            format!(
                "the published basis revision exceeds the supported {MAX_BASIS_REVISION} maximum"
            ),
        ));
    }
    Ok(())
}

fn canonical_head_json(head: &WorkingHead) -> Result<String, Failure> {
    let value = serde_json::to_string(head).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_INVARIANT",
            "revision-3 Quest basis head could not be serialized",
        )
    })?;
    if value.is_empty() || value.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_RESPONSE_LIMIT",
            "revision-3 Quest basis head exceeds its bounded transport limit",
        ));
    }
    Ok(value)
}

fn enforce_response_budget(response: &Value) -> Result<(), Failure> {
    let encoded = serde_json::to_vec(response).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_INVARIANT",
            "revision-3 Quest response could not be serialized",
        )
    })?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_RESPONSE_LIMIT",
            "revision-3 Quest response exceeds its bounded transport budget",
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
        "AUTHORING_REVISION3_QUEST_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and exactly current_project_json, game_root, quest_request_json, and root",
    )
}

fn input_changed() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_INPUT_CHANGED",
        "the native game generation changed during revision-3 Quest preparation",
    )
}

fn map_pristine_error(error: gore_mod::ModError) -> Failure {
    let message = error.to_string();
    if message.contains("RECOVERY_REQUIRED") {
        return Failure::new(
            "AUTHORING_REVISION3_QUEST_RECOVERY_REQUIRED",
            "an interrupted deployment must be recovered before Quest authoring",
        );
    }
    if message.contains("exceeds the") || message.contains("too large") {
        return Failure::new(
            "AUTHORING_REVISION3_QUEST_INPUT_LIMIT",
            "the pristine Shipping cache exceeds its bounded input limit",
        );
    }
    if message.contains("not a regular non-link file") {
        return Failure::new(
            "AUTHORING_REVISION3_QUEST_INPUT_UNSAFE",
            "the pristine Shipping cache is not a safe regular file",
        );
    }
    Failure::new(
        "AUTHORING_REVISION3_QUEST_PRISTINE_UNAVAILABLE",
        "the pristine Shipping cache could not be selected safely",
    )
}

fn map_catalog_error(error: CatalogError) -> Failure {
    match error {
        CatalogError::InvalidLimits(_) | CatalogError::LimitExceeded { .. } => Failure::new(
            "AUTHORING_REVISION3_QUEST_INPUT_LIMIT",
            "a native game-generation input exceeds its bounded resource limit",
        ),
        CatalogError::UnsafeInput(_) | CatalogError::OutputAliasesInput { .. } => Failure::new(
            "AUTHORING_REVISION3_QUEST_INPUT_UNSAFE",
            "a native game-generation input is unsafe",
        ),
        CatalogError::IdentityChanged(_) | CatalogError::SourceChanged { .. } => input_changed(),
        CatalogError::UnsupportedGeneration { .. } => Failure::new(
            "AUTHORING_REVISION3_QUEST_UNSUPPORTED_GENERATION",
            "the selected game does not match the supported pinned generation",
        ),
        CatalogError::Io { source, .. } if source.kind() == io::ErrorKind::NotFound => {
            Failure::new(
                "AUTHORING_REVISION3_QUEST_INPUT_MISSING",
                "a required native game-generation input does not exist",
            )
        }
        _ => Failure::new(
            "AUTHORING_REVISION3_QUEST_INPUT_UNAVAILABLE",
            "the native game generation could not be verified safely",
        ),
    }
}

fn map_source_read_error(error: SourceReadError) -> Failure {
    match error {
        SourceReadError::Missing => Failure::new(
            "AUTHORING_REVISION3_QUEST_INPUT_MISSING",
            "a required native game-generation input does not exist",
        ),
        SourceReadError::Unsafe => Failure::new(
            "AUTHORING_REVISION3_QUEST_INPUT_UNSAFE",
            "a native game-generation input is unsafe",
        ),
        SourceReadError::Limit => Failure::new(
            "AUTHORING_REVISION3_QUEST_INPUT_LIMIT",
            "a native game-generation input exceeds its bounded resource limit",
        ),
        SourceReadError::Changed => input_changed(),
        SourceReadError::Io => Failure::new(
            "AUTHORING_REVISION3_QUEST_INPUT_UNAVAILABLE",
            "a native game-generation input could not be read safely",
        ),
    }
}

fn map_inventory_error(error: StoryInventoryError) -> Failure {
    match error {
        StoryInventoryError::LimitExceeded { .. } | StoryInventoryError::SourcePairTooLarge => {
            Failure::new(
                "AUTHORING_REVISION3_QUEST_COLLISION_LIMIT",
                "the trusted collision inventory exceeds its bounded resource limit",
            )
        }
        StoryInventoryError::UnsupportedGeneration => Failure::new(
            "AUTHORING_REVISION3_QUEST_UNSUPPORTED_GENERATION",
            "the selected game does not match the supported pinned generation",
        ),
        StoryInventoryError::SourceLengthMismatch { .. }
        | StoryInventoryError::SourceDigestMismatch { .. }
        | StoryInventoryError::SourcePairSealMismatch
        | StoryInventoryError::RecollectedInventoryMismatch => input_changed(),
        _ => Failure::new(
            "AUTHORING_REVISION3_QUEST_INVENTORY_FAILED",
            "the trusted base-game collision inventory could not be rebuilt",
        ),
    }
}

fn map_basis_source_error(error: Revision3QuestCollisionSourceErrorV2) -> Failure {
    match error {
        Revision3QuestCollisionSourceErrorV2::Store(error) => map_store_error(error),
        Revision3QuestCollisionSourceErrorV2::CurrentSnapshotDrift => Failure::new(
            "AUTHORING_REVISION3_QUEST_HEAD_CONFLICT",
            "the current revision-3 project changed while its Quest source was prepared",
        ),
        Revision3QuestCollisionSourceErrorV2::Limit { .. }
        | Revision3QuestCollisionSourceErrorV2::TooManyPriorQuests { .. } => Failure::new(
            "AUTHORING_REVISION3_QUEST_COLLISION_LIMIT",
            "the exact current-project collision source exceeds its bounded resource limit",
        ),
        _ => Failure::new(
            "AUTHORING_REVISION3_QUEST_PROJECT_INVALID",
            "the exact current revision-3 project is not a valid Quest-authoring basis",
        ),
    }
}

fn map_capability_error(error: Revision3QuestCollisionCapabilityErrorV2) -> Failure {
    match error {
        Revision3QuestCollisionCapabilityErrorV2::TargetMismatch => Failure::new(
            "AUTHORING_REVISION3_QUEST_PROJECT_TARGET_MISMATCH",
            "the exact current project does not target the trusted game generation",
        ),
        Revision3QuestCollisionCapabilityErrorV2::Limit { .. } => Failure::new(
            "AUTHORING_REVISION3_QUEST_COLLISION_LIMIT",
            "the combined base/current collision authority exceeds its bounded resource limit",
        ),
        _ => Failure::new(
            "AUTHORING_REVISION3_QUEST_CAPABILITY_FAILED",
            "fresh revision-3 Quest collision authority could not be bound",
        ),
    }
}

fn map_artifact_error(_error: QuestCollisionCapabilityArtifactErrorV2) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_ARTIFACT_FAILED",
        "the fresh revision-3 Quest collision artifact could not be materialized",
    )
}

fn map_transaction_error(error: Revision3QuestDraftInsertErrorV3) -> Failure {
    use gore_story_inventory::Revision3QuestDraftInsertErrorV3 as E;
    match error {
        E::Request(_) => Failure::new(
            "AUTHORING_REVISION3_QUEST_REQUEST_INVALID",
            "quest_request_json is not an exact canonical revision-3 Quest request",
        ),
        E::ProjectTransport(Revision3QuestDraftProjectTransportErrorV3::InputTooLarge {
            ..
        }) => Failure::new(
            "AUTHORING_REVISION3_QUEST_PROJECT_LIMIT",
            "current_project_json exceeds the bounded project limit",
        ),
        E::ProjectTransport(
            Revision3QuestDraftProjectTransportErrorV3::CurrentProjectSealMismatch,
        ) => Failure::new(
            "AUTHORING_REVISION3_QUEST_HEAD_CONFLICT",
            "current_project_json differs from the exact published revision-3 project",
        ),
        E::ProjectTransport(Revision3QuestDraftProjectTransportErrorV3::InvalidProject(_)) => {
            Failure::new(
                "AUTHORING_REVISION3_QUEST_PROJECT_INVALID",
                "current_project_json is not the exact canonical published revision-3 project",
            )
        }
        E::Binding(_) => Failure::new(
            "AUTHORING_REVISION3_QUEST_HEAD_CONFLICT",
            "the Quest request, current project, and published basis do not match exactly",
        ),
        E::Conflict(_) => Failure::new(
            "AUTHORING_REVISION3_QUEST_REJECTED",
            "the revision-3 Quest Draft conflicts with trusted catalog or exact project state",
        ),
        _ => Failure::new(
            "AUTHORING_REVISION3_QUEST_TRANSACTION_FAILED",
            "the closed revision-3 Quest Draft transaction failed",
        ),
    }
}

fn map_persistence_error(error: Revision3QuestDraftPersistenceErrorV3) -> Failure {
    match error {
        Revision3QuestDraftPersistenceErrorV3::Store(error) => map_store_error(error),
        Revision3QuestDraftPersistenceErrorV3::BasisSource(error) => map_basis_source_error(error),
        Revision3QuestDraftPersistenceErrorV3::Validation(
            Revision3QuestDraftPersistenceValidationErrorV3::BasisHeadMismatch,
        ) => Failure::new(
            "AUTHORING_REVISION3_QUEST_HEAD_CONFLICT",
            "the published revision-3 project changed before Quest persistence",
        ),
        _ => Failure::new(
            "AUTHORING_REVISION3_QUEST_PERSISTENCE_FAILED",
            "the revision-3 Quest candidate failed structural persistence validation",
        ),
    }
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match &error {
        WorkingStoreError::InvalidLimits(_) => "AUTHORING_REVISION3_QUEST_STORE_LIMITS_INVALID",
        WorkingStoreError::MissingRoot(_) => "AUTHORING_REVISION3_QUEST_STORE_ROOT_MISSING",
        WorkingStoreError::UnsafePath { .. } => "AUTHORING_REVISION3_QUEST_STORE_PATH_UNSAFE",
        WorkingStoreError::LimitExceeded { .. } => "AUTHORING_REVISION3_QUEST_STORE_LIMIT",
        WorkingStoreError::HeadConflict { .. } => "AUTHORING_REVISION3_QUEST_HEAD_CONFLICT",
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_QUEST_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => "AUTHORING_REVISION3_QUEST_STORE_OBJECT_MISSING",
        WorkingStoreError::SealMismatch { .. } => "AUTHORING_REVISION3_QUEST_STORE_SEAL_MISMATCH",
        WorkingStoreError::Collision { .. } => "AUTHORING_REVISION3_QUEST_STORE_COLLISION",
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_QUEST_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_) => "AUTHORING_REVISION3_QUEST_STORE_INVARIANT",
        _ => "AUTHORING_REVISION3_QUEST_STORE_IO",
    };
    Failure::new(code, "the revision-3 working store operation failed")
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
    use std::path::{Path, PathBuf};

    use gore_authoring::{AssetVerification, ProjectRevision3, WorkingProjectStore};
    use gore_story_catalog::known_generation_v1;
    use gore_story_inventory::Revision3QuestDraftInsertRequestV3;
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;

    fn raw_request(payload: Value) -> String {
        json!({"command": COMMAND, "payload": payload}).to_string()
    }

    fn valid_shape() -> Value {
        json!({
            "current_project_json": "{}",
            "game_root": "C:/missing-game",
            "quest_request_json": "{}",
            "root": "C:/missing-store",
        })
    }

    fn project_json_at_revision(revision: u64) -> String {
        let project: ProjectRevision3 = serde_json::from_value(json!({
            "format": 2,
            "schema_revision": 3,
            "project_id": "03030303030303030303030303030303",
            "revision": revision,
            "meta": {"name": "Quest FFI", "version": "1.0.0", "author": "tests"},
            "target": {"executable": {
                "byte_len": 123,
                "sha256": "4545454545454545454545454545454545454545454545454545454545454545"
            }},
            "authoring_locales": [],
            "entities": {},
            "asset_store": {"assets": {}}
        }))
        .unwrap();
        project.to_canonical_json().unwrap()
    }

    fn published_store() -> (TempDir, String, Vec<u8>) {
        published_store_at_revision(7)
    }

    fn published_store_at_revision(revision: u64) -> (TempDir, String, Vec<u8>) {
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let project_json = project_json_at_revision(revision);
        let project = ProjectRevision3::from_json(&project_json).unwrap();
        let prepared = store.prepare_revision3_checkpoint(None, &project).unwrap();
        fs::write(temp.path().join("gore-project.json"), &prepared.head_bytes).unwrap();
        (temp, project_json, prepared.head_bytes)
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

    fn live_project_json() -> String {
        let generation = known_generation_v1();
        let project: ProjectRevision3 = serde_json::from_value(json!({
            "format": 2,
            "schema_revision": 3,
            "project_id": "93939393939393939393939393939393",
            "revision": 0,
            "meta": {"name": "Live Quest FFI", "version": "1.0.0", "author": "tests"},
            "target": {"executable": generation.executable},
            "authoring_locales": [],
            "entities": {},
            "asset_store": {"assets": {}}
        }))
        .unwrap();
        project.to_canonical_json().unwrap()
    }

    fn live_request_json(project: &ProjectRevision3, head: &WorkingHead, ordinal: u8) -> String {
        let request: Revision3QuestDraftInsertRequestV3 = serde_json::from_value(json!({
            "expected_head": head,
            "expected_project_id": project.project_id,
            "expected_revision": project.revision,
            "quest_id": format!("{:032x}", 0x80u128 + u128::from(ordinal) * 2),
            "script_module_id": format!("{:032x}", 0x81u128 + u128::from(ordinal) * 2),
            "display_name": format!("Native FFI Quest {ordinal}"),
            "intent": {
                "module_namespace": format!("GoreMods.Quests.NativeFfiQuest{ordinal}"),
                "technical_id": format!("GORE_NATIVE_FFI_QUEST_{ordinal}"),
                "text_helper": format!("GoreNativeFfiQuest{ordinal}Text"),
                "parent_catalog_id": "g1r:quest-parent:swampcamp_scchapter2",
                "giver_catalog_id": "g1r:npc:om_grd_asghan_263",
                "title": format!("Native FFI Quest {ordinal}"),
                "description": "Exercise the complete prepare-only native FFI path.",
                "objective_title": format!("Finish native FFI Quest {ordinal}"),
            }
        }))
        .unwrap();
        request.to_canonical_json().unwrap()
    }

    #[test]
    fn exact_raw_wire_rejects_duplicate_unknown_missing_and_wrongly_typed_fields() {
        let valid = raw_request(valid_shape());
        let parsed: PrepareQuestWirePayload = parse_exact_wire(&valid).unwrap();
        assert_eq!(parsed.current_project_json, "{}");

        let cases = [
            format!(
                "{{\"command\":\"{COMMAND}\",\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"game_root\":\"g\",\"quest_request_json\":\"{{}}\",\"root\":\"r\"}}}}"
            ),
            format!(
                "{{\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"current_project_json\":\"{{}}\",\"game_root\":\"g\",\"quest_request_json\":\"{{}}\",\"root\":\"r\"}}}}"
            ),
            raw_request(json!({
                "current_project_json": "{}", "game_root": "g",
                "quest_request_json": "{}", "root": "r", "authority": "forged"
            })),
            raw_request(json!({
                "game_root": "g", "quest_request_json": "{}", "root": "r"
            })),
            raw_request(json!({
                "current_project_json": {}, "game_root": "g",
                "quest_request_json": "{}", "root": "r"
            })),
        ];
        for input in cases {
            assert_eq!(
                prepare_revision3_quest_draft_v3_raw(&input)["error"]["code"],
                "AUTHORING_REVISION3_QUEST_REQUEST_INVALID"
            );
        }
    }

    #[test]
    fn public_dispatch_preserves_duplicate_rejection_for_the_raw_route() {
        let duplicate_payload = format!(
            "{{\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"game_root\":\"g\",\"game_root\":\"forged\",\"quest_request_json\":\"{{}}\",\"root\":\"r\"}}}}"
        );
        let response: Value =
            serde_json::from_str(&crate::execute_json(&duplicate_payload)).unwrap();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_QUEST_REQUEST_INVALID"
        );
    }

    #[test]
    fn nested_transports_and_paths_are_bounded_before_filesystem_access() {
        let mut payload = valid_shape();
        payload["root"] = Value::String("x".repeat(MAX_PATH_BYTES + 1));
        assert_eq!(
            prepare_revision3_quest_draft_v3_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_QUEST_REQUEST_INVALID"
        );

        let mut payload = valid_shape();
        payload["current_project_json"] = Value::String("x".repeat(MAX_PROJECT_JSON_BYTES + 1));
        assert_eq!(
            prepare_revision3_quest_draft_v3_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_QUEST_PROJECT_LIMIT"
        );

        let mut payload = valid_shape();
        payload["quest_request_json"] =
            Value::String("x".repeat(MAX_REVISION3_QUEST_DRAFT_REQUEST_JSON_BYTES_V3 + 1));
        assert_eq!(
            prepare_revision3_quest_draft_v3_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_QUEST_REQUEST_LIMIT"
        );
    }

    #[test]
    fn missing_native_generation_never_changes_the_published_store_head() {
        let (temp, project_json, fixed_head) = published_store();
        let missing_game = temp.path().join("missing-game");
        let response = prepare_revision3_quest_draft_v3_raw(&raw_request(json!({
            "current_project_json": project_json,
            "game_root": missing_game,
            "quest_request_json": "{}",
            "root": temp.path(),
        })));
        assert!(!response["ok"].as_bool().unwrap_or(false));
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            fixed_head
        );
        let encoded = response.to_string();
        assert!(!encoded.contains(temp.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn basis_revision_is_signed_wire_safe_and_rejected_before_any_cas_write() {
        assert!(validate_basis_revision(MAX_BASIS_REVISION).is_ok());
        assert_eq!(
            validate_basis_revision(MAX_BASIS_REVISION + 1)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_QUEST_REVISION_LIMIT"
        );

        for (revision, must_hit_revision_limit) in
            [(MAX_BASIS_REVISION, false), (MAX_BASIS_REVISION + 1, true)]
        {
            let (temp, project_json, fixed_head) = published_store_at_revision(revision);
            let before = snapshot_regular_files(temp.path());
            let response = prepare_revision3_quest_draft_v3_raw(&raw_request(json!({
                "current_project_json": project_json,
                "game_root": temp.path().join("missing-game"),
                "quest_request_json": "{}",
                "root": temp.path(),
            })));
            if must_hit_revision_limit {
                assert_eq!(
                    response["error"]["code"],
                    "AUTHORING_REVISION3_QUEST_REVISION_LIMIT"
                );
            } else {
                assert_ne!(
                    response["error"]["code"],
                    "AUTHORING_REVISION3_QUEST_REVISION_LIMIT"
                );
            }
            assert_eq!(
                fs::read(temp.path().join("gore-project.json")).unwrap(),
                fixed_head
            );
            assert_eq!(snapshot_regular_files(temp.path()), before);
            assert!(!response
                .to_string()
                .contains(temp.path().to_string_lossy().as_ref()));
        }
    }

    #[test]
    fn response_contract_exposes_only_structural_unpublished_statuses() {
        let head = WorkingHead {
            store_format: gore_authoring::WorkingStoreFormat,
            snapshot: gore_authoring::ContentSeal {
                byte_len: 1,
                sha256: gore_authoring::Sha256Digest::from_bytes([7; 32]),
            },
        };
        let encoded = canonical_head_json(&head).unwrap();
        assert!(encoded.contains("snapshot"));
        let response = json!({
            "ok": true,
            "outcome": "prepared_unpublished",
            "build_status": "blocked",
            "runtime_status": "runtime_unqualified",
            "artifact_authority": "not_granted",
            "source_inspection": "fresh_capability_required",
            "publication_status": "not_supported",
        });
        enforce_response_budget(&response).unwrap();
        assert_ne!(response["publication_status"], "published");
        assert!(response.get("compile_ready").is_none());
        assert!(response.get("runtime_qualified").is_none());
        assert!(response.get("artifact_json").is_none());
    }

    #[test]
    fn working_store_and_game_roots_must_be_disjoint_before_persistence() {
        let temp = TempDir::new().unwrap();
        let store_root = temp.path().join("store");
        let game_root = temp.path().join("game");
        fs::create_dir(&store_root).unwrap();
        fs::create_dir(&game_root).unwrap();
        let store = WorkingProjectStore::open_existing(&store_root, ffi_store_limits()).unwrap();
        ensure_store_is_outside_game(&store, &game_root).unwrap();

        let nested_game = store_root.join("game");
        fs::create_dir(&nested_game).unwrap();
        assert_eq!(
            ensure_store_is_outside_game(&store, &nested_game)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_QUEST_STORE_GAME_ALIAS"
        );

        let nested_store_root = game_root.join("project");
        fs::create_dir(&nested_store_root).unwrap();
        let nested_store =
            WorkingProjectStore::open_existing(&nested_store_root, ffi_store_limits()).unwrap();
        assert_eq!(
            ensure_store_is_outside_game(&nested_store, &game_root)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_QUEST_STORE_GAME_ALIAS"
        );

        // Supplying `.../G1R` directly still denotes the parent installation. A sibling project
        // store under that parent must not bypass the guard. Component matching is deliberately
        // ASCII-case-insensitive for Windows paths.
        let install_root = temp.path().join("install");
        let direct_g1r = install_root.join("g1r");
        let sibling_store_root = install_root.join("projects");
        fs::create_dir_all(&direct_g1r).unwrap();
        fs::create_dir(&sibling_store_root).unwrap();
        let sibling_store =
            WorkingProjectStore::open_existing(&sibling_store_root, ffi_store_limits()).unwrap();
        assert_eq!(semantic_install_root(&direct_g1r), install_root);
        assert_eq!(
            ensure_store_is_outside_game(&sibling_store, &direct_g1r)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_QUEST_STORE_GAME_ALIAS"
        );
    }

    #[test]
    fn g1r_root_resolver_is_ascii_case_insensitive_and_never_double_appends() {
        let install = Path::new("C:/Games/Gothic");
        assert_eq!(resolve_g1r_root(install), install.join("G1R"));
        for component in ["G1R", "g1r", "G1r", "g1R"] {
            let direct = install.join(component);
            assert_eq!(resolve_g1r_root(&direct), direct);
            assert_ne!(resolve_g1r_root(&direct), direct.join("G1R"));
        }
        assert_eq!(
            resolve_g1r_root(&install.join("G1R-backup")),
            install.join("G1R-backup").join("G1R")
        );
    }

    #[test]
    #[ignore = "requires GORE_STORY_GAME_ROOT pointing at the pinned supported game generation"]
    fn live_native_path_prepares_two_quests_and_never_publishes_either_candidate() {
        let game_root = std::env::var("GORE_STORY_GAME_ROOT")
            .expect("set GORE_STORY_GAME_ROOT to run the live revision-3 Quest FFI test");
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let base_json = live_project_json();
        let base = ProjectRevision3::from_json(&base_json).unwrap();
        let published = store.prepare_revision3_checkpoint(None, &base).unwrap();
        fs::write(temp.path().join("gore-project.json"), &published.head_bytes).unwrap();

        let first = prepare_revision3_quest_draft_v3_raw(&raw_request(json!({
            "current_project_json": base_json,
            "game_root": game_root,
            "quest_request_json": live_request_json(&base, &published.head, 1),
            "root": temp.path(),
        })));
        assert_eq!(first["ok"], true, "{first}");
        assert_eq!(first["outcome"], "prepared_unpublished");
        assert_eq!(first["revision"], 1);
        assert_eq!(first["build_status"], "blocked");
        assert_eq!(first["runtime_status"], "runtime_unqualified");
        assert_eq!(first["artifact_authority"], "not_granted");
        assert_eq!(first["publication_status"], "not_supported");
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            published.head_bytes
        );
        let first_head_json = first["head_json"].as_str().unwrap();
        let first_opened = store
            .open_revision3_head_bytes(first_head_json.as_bytes(), AssetVerification::Full)
            .unwrap();
        assert_eq!(
            first_opened.project.to_canonical_json().unwrap(),
            first["project_json"].as_str().unwrap()
        );

        // Simulate the managed session's separate exact-CAS publication, then prove that the
        // same native command can prepare a second Quest against the new exact basis.
        fs::write(temp.path().join("gore-project.json"), first_head_json).unwrap();
        let first_project_json = first["project_json"].as_str().unwrap();
        let first_project = ProjectRevision3::from_json(first_project_json).unwrap();
        let second = prepare_revision3_quest_draft_v3_raw(&raw_request(json!({
            "current_project_json": first_project_json,
            "game_root": std::env::var("GORE_STORY_GAME_ROOT").unwrap(),
            "quest_request_json": live_request_json(&first_project, &first_opened.head, 2),
            "root": temp.path(),
        })));
        assert_eq!(second["ok"], true, "{second}");
        assert_eq!(second["revision"], 2);
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            first_head_json.as_bytes()
        );
        let second_opened = store
            .open_revision3_head_bytes(
                second["head_json"].as_str().unwrap().as_bytes(),
                AssetVerification::Full,
            )
            .unwrap();
        assert_eq!(second_opened.project.entities.len(), 4);
        let encoded = second.to_string();
        assert!(!encoded.contains(temp.path().to_string_lossy().as_ref()));
        assert!(!encoded.contains(&std::env::var("GORE_STORY_GAME_ROOT").unwrap()));
        assert!(second.get("artifact_json").is_none());
    }
}
