//! Native, prepare-only orchestration for one existing revision-3 Quest context edit.
//!
//! The caller supplies only exact project/request transports plus a selected working Store and
//! game root. Native code rebuilds the trusted Story catalog and collision inventory, binds them
//! to the exact fully reopened project, resolves parent/giver catalog IDs inside that authority,
//! and prepares a fully reopened immutable candidate. It never replaces the fixed project head,
//! imports an artifact, compiles, deploys, writes a save, or grants runtime authority.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use gore_authoring::{
    AssetVerification, Revision3EntityPayload, Revision3QuestCollisionSourceErrorV2, WorkingHead,
    WorkingProjectStore, WorkingStoreError, WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
};
use gore_story_catalog::{
    build_known_catalog_with_shipping_snapshot, CatalogError, GenerationInputLimits,
    StoryCatalogFile,
};
use gore_story_inventory::{
    apply_revision3_quest_context_edit_transaction_v1, build_base_game_inventory,
    QuestCollisionCapabilityArtifactErrorV2, Revision3QuestCollisionCapabilityErrorV2,
    Revision3QuestContextEditBindingErrorV1, Revision3QuestContextEditBuildStatusV1,
    Revision3QuestContextEditConflictV1, Revision3QuestContextEditErrorV1,
    Revision3QuestContextEditProjectTransportErrorV1, Revision3QuestContextEditPublicationStatusV1,
    Revision3QuestContextEditRequestV1, Revision3QuestContextEditRuntimeStatusV1,
    StoryInventoryError, VerifiedRevision3QuestCollisionCapabilityV2, MAX_BINDS_CACHE_SOURCE_BYTES,
    MAX_REVISION3_QUEST_CONTEXT_EDIT_REQUEST_JSON_BYTES_V1,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

use crate::authoring_source_io::{read_source_no_follow, SourceReadError};
use crate::err;

pub(super) const COMMAND: &str = "authoring_store_prepare_revision3_quest_context_edit_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_BASIS_REVISION: u64 = i64::MAX as u64 - 1;
const MAX_WIRE_BYTES: usize = MAX_PROJECT_JSON_BYTES * 2
    + MAX_REVISION3_QUEST_CONTEXT_EDIT_REQUEST_JSON_BYTES_V1 * 2
    + MAX_PATH_BYTES * 12
    + 8 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PrepareQuestContextWirePayload {
    current_project_json: String,
    game_root: String,
    quest_context_request_json: String,
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

pub(super) fn prepare_revision3_quest_context_edit_v1_raw(input: &str) -> Value {
    prepare_revision3_quest_context_edit_v1_inner(input).unwrap_or_else(Failure::response)
}

fn prepare_revision3_quest_context_edit_v1_inner(input: &str) -> Result<Value, Failure> {
    prepare_revision3_quest_context_edit_v1_inner_with_test_seams(input, || {}, || {})
}

fn prepare_revision3_quest_context_edit_v1_inner_with_test_seams<B, F>(
    input: &str,
    before_checkpoint: B,
    final_guard: F,
) -> Result<Value, Failure>
where
    B: FnOnce(),
    F: FnOnce(),
{
    let payload: PrepareQuestContextWirePayload = parse_exact_wire(input)?;
    validate_payload(&payload)?;
    let request =
        Revision3QuestContextEditRequestV1::from_json(&payload.quest_context_request_json)
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

    let canonical_basis = basis.project.to_canonical_json().map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_STORE_INVARIANT",
            "the exact current revision-3 project could not be serialized canonically",
        )
    })?;
    if canonical_basis.as_bytes() != payload.current_project_json.as_bytes() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_PROJECT_CONFLICT",
            "current_project_json differs from the exact published revision-3 project",
        ));
    }
    bind_request_to_basis(&basis.head, &basis.project, &request)?;

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
    if capability.story_catalog_seal() != &request.expected_story_catalog_seal {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_CATALOG_CONFLICT",
            "the trusted Story catalog changed after the Quest context choices were shown",
        ));
    }
    let story_catalog_seal = capability.story_catalog_seal().clone();
    let prepared_artifact = capability.prepare_artifact().map_err(map_artifact_error)?;
    let outcome = apply_revision3_quest_context_edit_transaction_v1(
        prepared_artifact,
        &payload.current_project_json,
        &payload.quest_context_request_json,
    )
    .map_err(map_transaction_error)?;

    require_signed_serializable(outcome.project())?;
    if outcome.basis_head() != &basis.head
        || outcome.quest_id() != request.quest_id
        || outcome.project().project_id != basis.project.project_id
        || outcome.project().target != basis.project.target
        || outcome.project().revision != basis.project.revision + 1
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_INVARIANT",
            "the Quest context transaction changed its exact project/request binding",
        ));
    }
    let basis_quest = basis
        .project
        .entities
        .get(&request.quest_id)
        .ok_or_else(|| quest_conflict("the bound Quest disappeared during preparation"))?;
    let basis_quest_payload = match &basis_quest.payload {
        Revision3EntityPayload::QuestDraft(quest) => quest,
        _ => {
            return Err(quest_conflict(
                "the bound Quest changed kind during preparation",
            ));
        }
    };
    let module_id = basis_quest_payload.script_module.id;
    let basis_module = basis.project.entities.get(&module_id).ok_or_else(|| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_PROJECT_INVALID",
            "the bound Quest module disappeared during preparation",
        )
    })?;
    if outcome.script_module_id() != module_id
        || outcome.quest_revision() != request.expected_quest_revision + 1
        || outcome.script_module_revision() != basis_module.revision + 1
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_INVARIANT",
            "the Quest context transaction returned unexpected entity revisions",
        ));
    }
    match outcome.build_status() {
        Revision3QuestContextEditBuildStatusV1::Blocked => {}
    }
    match outcome.runtime_status() {
        Revision3QuestContextEditRuntimeStatusV1::RuntimeUnqualified => {}
    }
    match outcome.publication_status() {
        Revision3QuestContextEditPublicationStatusV1::NotSupported => {}
    }

    revalidate_game_inputs(&catalog, &game_root, &shipping)?;
    ensure_store_is_outside_game(&store, &game_root)?;
    before_checkpoint();
    let prepared = store
        .prepare_revision3_checkpoint(Some(&basis.head), outcome.project())
        .map_err(map_store_error)?;
    let reopened = store
        .open_revision3_head_bytes(&prepared.head_bytes, AssetVerification::Full)
        .map_err(map_store_error)?;
    if reopened.head != prepared.head || reopened.project != *outcome.project() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_STORE_INVARIANT",
            "the prepared Quest context checkpoint did not fully reopen exactly",
        ));
    }
    let reopened_json = reopened.project.to_canonical_json().map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_STORE_INVARIANT",
            "the reopened Quest context candidate could not be serialized",
        )
    })?;
    if reopened_json != outcome.canonical_project_json() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_STORE_INVARIANT",
            "the reopened Quest context candidate changed canonical bytes",
        ));
    }

    final_guard();
    require_fixed_basis(&store, &basis.head, &basis.project)?;
    revalidate_game_inputs(&catalog, &game_root, &shipping)?;

    let basis_head_json = canonical_head_json(&basis.head)?;
    let candidate_head_json = String::from_utf8(prepared.head_bytes).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_STORE_INVARIANT",
            "the prepared Quest context head is not UTF-8 JSON",
        )
    })?;
    if candidate_head_json.is_empty() || candidate_head_json.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_RESPONSE_LIMIT",
            "the prepared Quest context head exceeds its bounded transport limit",
        ));
    }
    require_signed_serializable(&prepared.head)?;
    let response = json!({
        "ok": true,
        "outcome": "prepared_unpublished",
        "basis_head_json": basis_head_json,
        "head_json": candidate_head_json,
        "project_json": outcome.canonical_project_json(),
        "project_id": outcome.project().project_id.to_string(),
        "revision": outcome.project().revision,
        "quest_id": outcome.quest_id().to_string(),
        "module_id": outcome.script_module_id().to_string(),
        "quest_revision": outcome.quest_revision(),
        "module_revision": outcome.script_module_revision(),
        "story_catalog_seal": story_catalog_seal,
        "parent_catalog_id": request.parent_catalog_id,
        "giver_catalog_id": request.giver_catalog_id,
        "build_status": "blocked",
        "runtime_status": "runtime_unqualified",
        "publication_status": "not_supported",
    });
    enforce_response_budget(&response)?;

    require_fixed_basis(&store, &basis.head, &basis.project)?;
    revalidate_game_inputs(&catalog, &game_root, &shipping)?;
    Ok(response)
}

fn parse_exact_wire<P>(input: &str) -> Result<P, Failure>
where
    P: DeserializeOwned + Serialize,
{
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_INPUT_LIMIT",
            format!("Quest context request exceeds the {MAX_WIRE_BYTES}-byte wire limit"),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    let canonical = serde_json::to_string(&request).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_INVARIANT",
            "the Quest context outer request could not be serialized",
        )
    })?;
    if canonical != input {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_payload(payload: &PrepareQuestContextWirePayload) -> Result<(), Failure> {
    validate_path(&payload.root)?;
    validate_path(&payload.game_root)?;
    if payload.current_project_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_PROJECT_LIMIT",
            format!("current_project_json exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ));
    }
    if payload.quest_context_request_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.quest_context_request_json.len()
        > MAX_REVISION3_QUEST_CONTEXT_EDIT_REQUEST_JSON_BYTES_V1
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_REQUEST_LIMIT",
            format!(
                "quest_context_request_json exceeds the {MAX_REVISION3_QUEST_CONTEXT_EDIT_REQUEST_JSON_BYTES_V1}-byte limit"
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

fn bind_request_to_basis(
    head: &WorkingHead,
    project: &gore_authoring::ProjectRevision3,
    request: &Revision3QuestContextEditRequestV1,
) -> Result<(), Failure> {
    if request.expected_head != *head {
        return Err(head_conflict());
    }
    if request.expected_project_id != project.project_id
        || request.expected_revision != project.revision
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_PROJECT_CONFLICT",
            "the Quest context request project differs from the exact published project",
        ));
    }
    let Some(entity) = project.entities.get(&request.quest_id) else {
        return Err(quest_conflict(
            "the requested Quest does not exist in the exact published project",
        ));
    };
    let Revision3EntityPayload::QuestDraft(quest) = &entity.payload else {
        return Err(quest_conflict(
            "the requested entity is not a Quest in the exact published project",
        ));
    };
    if entity.revision != request.expected_quest_revision {
        return Err(quest_conflict(
            "the requested Quest revision differs from the exact published entity revision",
        ));
    }
    if quest.script_module.project_id != project.project_id {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_PROJECT_INVALID",
            "the exact Quest has a foreign owned module binding",
        ));
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
            "AUTHORING_REVISION3_QUEST_CONTEXT_STORE_PATH_UNSAFE",
            "the revision-3 working-store root could not be resolved safely",
        )
    })?;
    let semantic_install_root =
        fs::canonicalize(semantic_install_root(game_root)).map_err(|_| {
            Failure::new(
                "AUTHORING_REVISION3_QUEST_CONTEXT_INPUT_UNAVAILABLE",
                "the selected game installation root could not be resolved safely",
            )
        })?;
    if store_root.starts_with(&semantic_install_root)
        || semantic_install_root.starts_with(&store_root)
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_STORE_GAME_ALIAS",
            "the working-store root and selected game installation must be disjoint",
        ));
    }
    Ok(())
}

fn semantic_install_root(game_root: &Path) -> PathBuf {
    if game_root.file_name().is_some_and(is_g1r_component) {
        game_root
            .parent()
            .filter(|value| !value.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    } else {
        game_root.to_path_buf()
    }
}

fn is_g1r_component(value: &std::ffi::OsStr) -> bool {
    value.as_encoded_bytes().eq_ignore_ascii_case(b"G1R")
}

fn require_fixed_basis(
    store: &WorkingProjectStore,
    expected_head: &WorkingHead,
    expected_project: &gore_authoring::ProjectRevision3,
) -> Result<(), Failure> {
    let current = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if &current.head != expected_head || &current.project != expected_project {
        return Err(head_conflict());
    }
    Ok(())
}

fn validate_basis_revision(revision: u64) -> Result<(), Failure> {
    if revision > MAX_BASIS_REVISION {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_REVISION_LIMIT",
            format!("the published basis revision exceeds {MAX_BASIS_REVISION}"),
        ));
    }
    Ok(())
}

fn require_signed_serializable(value: &impl Serialize) -> Result<(), Failure> {
    let value = serde_json::to_value(value).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_INVARIANT",
            "a Quest context wire value could not be inspected",
        )
    })?;
    require_signed_json_value(&value)
}

fn require_signed_json_value(value: &Value) -> Result<(), Failure> {
    match value {
        Value::Number(number) if number.as_u64().is_some_and(|value| value > i64::MAX as u64) => {
            Err(Failure::new(
                "AUTHORING_REVISION3_QUEST_CONTEXT_SIGNED_WIRE_LIMIT",
                "a Quest context wire integer exceeds the signed 64-bit transport range",
            ))
        }
        Value::Array(values) => values.iter().try_for_each(require_signed_json_value),
        Value::Object(values) => values.values().try_for_each(require_signed_json_value),
        _ => Ok(()),
    }
}

fn canonical_head_json(head: &WorkingHead) -> Result<String, Failure> {
    require_signed_serializable(head)?;
    let value = serde_json::to_string(head).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_INVARIANT",
            "the Quest context basis head could not be serialized",
        )
    })?;
    if value.is_empty() || value.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_RESPONSE_LIMIT",
            "the Quest context basis head exceeds its bounded transport limit",
        ));
    }
    Ok(value)
}

fn enforce_response_budget(response: &Value) -> Result<(), Failure> {
    let encoded = serde_json::to_vec(response).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_INVARIANT",
            "the Quest context response could not be serialized",
        )
    })?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_RESPONSE_LIMIT",
            "the Quest context response exceeds its bounded transport budget",
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
        "AUTHORING_REVISION3_QUEST_CONTEXT_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and exactly current_project_json, game_root, quest_context_request_json, and root",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_CONTEXT_HEAD_CONFLICT",
        "the published revision-3 head changed or differs from the Quest context request",
    )
}

fn quest_conflict(message: &'static str) -> Failure {
    Failure::new("AUTHORING_REVISION3_QUEST_CONTEXT_QUEST_CONFLICT", message)
}

fn input_changed() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_CONTEXT_INPUT_CHANGED",
        "the native game generation changed during Quest context preparation",
    )
}

fn map_request_error(error: impl std::fmt::Display) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_CONTEXT_REQUEST_INVALID",
        format!("the exact Quest context request is invalid: {error}"),
    )
}

fn map_pristine_error(error: gore_mod::ModError) -> Failure {
    let message = error.to_string();
    if message.contains("RECOVERY_REQUIRED") {
        return Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_RECOVERY_REQUIRED",
            "an interrupted deployment must be recovered before Quest context authoring",
        );
    }
    if message.contains("exceeds the") || message.contains("too large") {
        return Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_INPUT_LIMIT",
            "the pristine Shipping cache exceeds its bounded input limit",
        );
    }
    if message.contains("not a regular non-link file") {
        return Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_INPUT_UNSAFE",
            "the pristine Shipping cache is not a safe regular file",
        );
    }
    Failure::new(
        "AUTHORING_REVISION3_QUEST_CONTEXT_PRISTINE_UNAVAILABLE",
        "the pristine Shipping cache could not be selected safely",
    )
}

fn map_catalog_error(error: CatalogError) -> Failure {
    match error {
        CatalogError::InvalidLimits(_) | CatalogError::LimitExceeded { .. } => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_INPUT_LIMIT",
            "a native game-generation input exceeds its bounded resource limit",
        ),
        CatalogError::UnsafeInput(_) | CatalogError::OutputAliasesInput { .. } => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_INPUT_UNSAFE",
            "a native game-generation input is unsafe",
        ),
        CatalogError::IdentityChanged(_) | CatalogError::SourceChanged { .. } => input_changed(),
        CatalogError::UnsupportedGeneration { .. } => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_UNSUPPORTED_GENERATION",
            "the selected game does not match the supported pinned generation",
        ),
        CatalogError::Io { source, .. } if source.kind() == io::ErrorKind::NotFound => {
            Failure::new(
                "AUTHORING_REVISION3_QUEST_CONTEXT_INPUT_MISSING",
                "a required native game-generation input does not exist",
            )
        }
        _ => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_INPUT_UNAVAILABLE",
            "the native game generation could not be verified safely",
        ),
    }
}

fn map_source_read_error(error: SourceReadError) -> Failure {
    match error {
        SourceReadError::Missing => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_INPUT_MISSING",
            "a required native game-generation input does not exist",
        ),
        SourceReadError::Unsafe => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_INPUT_UNSAFE",
            "a native game-generation input is unsafe",
        ),
        SourceReadError::Limit => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_INPUT_LIMIT",
            "a native game-generation input exceeds its bounded resource limit",
        ),
        SourceReadError::Changed => input_changed(),
        SourceReadError::Io => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_INPUT_UNAVAILABLE",
            "a native game-generation input could not be read safely",
        ),
    }
}

fn map_inventory_error(error: StoryInventoryError) -> Failure {
    match error {
        StoryInventoryError::LimitExceeded { .. } | StoryInventoryError::SourcePairTooLarge => {
            Failure::new(
                "AUTHORING_REVISION3_QUEST_CONTEXT_COLLISION_LIMIT",
                "the trusted collision inventory exceeds its bounded resource limit",
            )
        }
        StoryInventoryError::UnsupportedGeneration => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_UNSUPPORTED_GENERATION",
            "the selected game does not match the supported pinned generation",
        ),
        StoryInventoryError::SourceLengthMismatch { .. }
        | StoryInventoryError::SourceDigestMismatch { .. }
        | StoryInventoryError::SourcePairSealMismatch
        | StoryInventoryError::RecollectedInventoryMismatch => input_changed(),
        _ => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_INVENTORY_FAILED",
            "the trusted base-game collision inventory could not be rebuilt",
        ),
    }
}

fn map_basis_source_error(error: Revision3QuestCollisionSourceErrorV2) -> Failure {
    match error {
        Revision3QuestCollisionSourceErrorV2::Store(error) => map_store_error(error),
        Revision3QuestCollisionSourceErrorV2::CurrentSnapshotDrift => head_conflict(),
        Revision3QuestCollisionSourceErrorV2::Limit { .. }
        | Revision3QuestCollisionSourceErrorV2::TooManyPriorQuests { .. } => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_COLLISION_LIMIT",
            "the exact current-project Quest source exceeds its bounded resource limit",
        ),
        _ => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_PROJECT_INVALID",
            "the exact current revision-3 project is not a valid Quest context basis",
        ),
    }
}

fn map_capability_error(error: Revision3QuestCollisionCapabilityErrorV2) -> Failure {
    match error {
        Revision3QuestCollisionCapabilityErrorV2::TargetMismatch => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_TARGET_CONFLICT",
            "the exact current project does not target the trusted game generation",
        ),
        Revision3QuestCollisionCapabilityErrorV2::PriorQuestParentDrift { .. }
        | Revision3QuestCollisionCapabilityErrorV2::PriorQuestGiverDrift { .. } => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_CATALOG_CONFLICT",
            "an existing Quest context no longer matches the selected exact Story catalog",
        ),
        Revision3QuestCollisionCapabilityErrorV2::Limit { .. } => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_COLLISION_LIMIT",
            "the combined base/current collision authority exceeds its bounded resource limit",
        ),
        _ => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_CAPABILITY_FAILED",
            "fresh Quest context authority could not be bound",
        ),
    }
}

fn map_artifact_error(_error: QuestCollisionCapabilityArtifactErrorV2) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_QUEST_CONTEXT_ARTIFACT_FAILED",
        "the fresh Quest context capability could not be sealed in memory",
    )
}

fn map_transaction_error(error: Revision3QuestContextEditErrorV1) -> Failure {
    use gore_story_inventory::Revision3QuestContextEditErrorV1 as E;
    match error {
        E::Capability(_) => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_CAPABILITY_FAILED",
            "the prepared Quest context capability failed exact verification",
        ),
        E::Request(_) => map_request_error("request transport rejected"),
        E::ProjectTransport(Revision3QuestContextEditProjectTransportErrorV1::InputTooLarge {
            ..
        }) => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_PROJECT_LIMIT",
            "current_project_json exceeds the bounded project limit",
        ),
        E::ProjectTransport(
            Revision3QuestContextEditProjectTransportErrorV1::CurrentProjectSealMismatch,
        ) => head_conflict(),
        E::ProjectTransport(Revision3QuestContextEditProjectTransportErrorV1::InvalidProject(
            _,
        )) => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_PROJECT_INVALID",
            "current_project_json is not the exact canonical published project",
        ),
        E::Binding(Revision3QuestContextEditBindingErrorV1::CurrentHeadMismatch) => head_conflict(),
        E::Binding(
            Revision3QuestContextEditBindingErrorV1::ProjectIdentityMismatch
            | Revision3QuestContextEditBindingErrorV1::ProjectRevisionMismatch,
        ) => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_PROJECT_CONFLICT",
            "the Quest context request project differs from the exact capability basis",
        ),
        E::Binding(Revision3QuestContextEditBindingErrorV1::ProjectTargetMismatch) => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_TARGET_CONFLICT",
            "the exact project target differs from the trusted game generation",
        ),
        E::Binding(Revision3QuestContextEditBindingErrorV1::StoryCatalogSealMismatch) => {
            Failure::new(
                "AUTHORING_REVISION3_QUEST_CONTEXT_CATALOG_CONFLICT",
                "the trusted Story catalog changed after the Quest context choices were shown",
            )
        }
        E::Conflict(conflict) => map_transaction_conflict(conflict),
        E::Generation(_) => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_PROJECT_INVALID",
            "the existing Quest context could not be regenerated deterministically",
        ),
        E::CanonicalReopen(_) | E::CanonicalReopenMismatch => Failure::new(
            "AUTHORING_REVISION3_QUEST_CONTEXT_INVARIANT",
            "the Quest context candidate failed exact canonical reopen",
        ),
    }
}

fn map_transaction_conflict(error: Revision3QuestContextEditConflictV1) -> Failure {
    let text = error.to_string();
    let code = match &error {
        Revision3QuestContextEditConflictV1::NoChanges => {
            "AUTHORING_REVISION3_QUEST_CONTEXT_NO_CHANGES"
        }
        Revision3QuestContextEditConflictV1::CatalogSelection(_) => {
            "AUTHORING_REVISION3_QUEST_CONTEXT_CATALOG_CONFLICT"
        }
        Revision3QuestContextEditConflictV1::InvalidQuestEntity { .. }
        | Revision3QuestContextEditConflictV1::QuestRevisionConflict { .. } => {
            "AUTHORING_REVISION3_QUEST_CONTEXT_QUEST_CONFLICT"
        }
        Revision3QuestContextEditConflictV1::ProjectRevisionOverflow
        | Revision3QuestContextEditConflictV1::QuestRevisionOverflow { .. }
        | Revision3QuestContextEditConflictV1::ScriptModuleRevisionOverflow { .. } => {
            "AUTHORING_REVISION3_QUEST_CONTEXT_REVISION_LIMIT"
        }
        Revision3QuestContextEditConflictV1::CandidateTooLarge { .. } => {
            "AUTHORING_REVISION3_QUEST_CONTEXT_PROJECT_LIMIT"
        }
        Revision3QuestContextEditConflictV1::InvalidQuestClosure { .. }
        | Revision3QuestContextEditConflictV1::OwnedModuleDrift { .. }
        | Revision3QuestContextEditConflictV1::TechnicalIdentityChanged
        | Revision3QuestContextEditConflictV1::CandidateNotPersistable { .. } => {
            "AUTHORING_REVISION3_QUEST_CONTEXT_PROJECT_INVALID"
        }
        Revision3QuestContextEditConflictV1::ZeroQuestId
        | Revision3QuestContextEditConflictV1::InvalidQuestContext { .. } => {
            "AUTHORING_REVISION3_QUEST_CONTEXT_REQUEST_REJECTED"
        }
    };
    Failure::new(code, text)
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::InvalidLimits(_) => {
            "AUTHORING_REVISION3_QUEST_CONTEXT_STORE_LIMITS_INVALID"
        }
        WorkingStoreError::MissingRoot(_) => "AUTHORING_REVISION3_QUEST_CONTEXT_STORE_ROOT_MISSING",
        WorkingStoreError::UnsafePath { .. } => {
            "AUTHORING_REVISION3_QUEST_CONTEXT_STORE_PATH_UNSAFE"
        }
        WorkingStoreError::LimitExceeded { .. } => "AUTHORING_REVISION3_QUEST_CONTEXT_STORE_LIMIT",
        WorkingStoreError::HeadConflict { .. } => "AUTHORING_REVISION3_QUEST_CONTEXT_HEAD_CONFLICT",
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_QUEST_CONTEXT_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => {
            "AUTHORING_REVISION3_QUEST_CONTEXT_STORE_OBJECT_MISSING"
        }
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_QUEST_CONTEXT_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => "AUTHORING_REVISION3_QUEST_CONTEXT_STORE_COLLISION",
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_QUEST_CONTEXT_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_) => "AUTHORING_REVISION3_QUEST_CONTEXT_STORE_INVARIANT",
        _ => "AUTHORING_REVISION3_QUEST_CONTEXT_STORE_IO",
    };
    Failure::new(code, "the revision-3 working Store operation failed")
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

    use gore_authoring::{ProjectRevision3, Revision3EntityPayload, WorkingProjectStore};
    use gore_story_catalog::ContentSeal;
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
            "quest_context_request_json": "{}",
            "root": "C:/missing-store",
        })
    }

    fn project_json_at_revision(revision: u64) -> String {
        let project: ProjectRevision3 = serde_json::from_value(json!({
            "format": 2,
            "schema_revision": 3,
            "project_id": "03030303030303030303030303030303",
            "revision": revision,
            "meta": {"name": "Quest Context FFI", "version": "1.0.0", "author": "tests"},
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

    fn live_project_json(executable: &ContentSeal) -> String {
        let project: ProjectRevision3 = serde_json::from_value(json!({
            "format": 2,
            "schema_revision": 3,
            "project_id": "93939393939393939393939393939393",
            "revision": 0,
            "meta": {"name": "Live Quest Context FFI", "version": "1.0.0", "author": "tests"},
            "target": {"executable": executable},
            "authoring_locales": [],
            "entities": {},
            "asset_store": {"assets": {}}
        }))
        .unwrap();
        project.to_canonical_json().unwrap()
    }

    fn live_draft_request_json(project: &ProjectRevision3, head: &WorkingHead) -> String {
        let request: Revision3QuestDraftInsertRequestV3 = serde_json::from_value(json!({
            "expected_head": head,
            "expected_project_id": project.project_id,
            "expected_revision": project.revision,
            "quest_id": "80808080808080808080808080808080",
            "script_module_id": "81818181818181818181818181818181",
            "display_name": "Native Context Fixture",
            "intent": {
                "module_namespace": "GoreMods.Quests.NativeContextFixture",
                "technical_id": "GORE_NATIVE_CONTEXT_FIXTURE",
                "text_helper": "GoreNativeContextFixtureText",
                "parent_catalog_id": "g1r:quest-parent:swampcamp_scchapter2",
                "giver_catalog_id": "g1r:npc:om_grd_asghan_263",
                "title": "Native Context Fixture",
                "description": "The original exact Quest context.",
                "objective_title": "Finish the context fixture"
            }
        }))
        .unwrap();
        request.to_canonical_json().unwrap()
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

    fn request_json(project: &ProjectRevision3, head: &WorkingHead, quest_revision: u64) -> String {
        let request: Revision3QuestContextEditRequestV1 = serde_json::from_value(json!({
            "expected_head": head,
            "expected_project_id": project.project_id,
            "expected_revision": project.revision,
            "expected_story_catalog_seal": {
                "byte_len": 1,
                "sha256": "1111111111111111111111111111111111111111111111111111111111111111"
            },
            "quest_id": "21212121212121212121212121212121",
            "expected_quest_revision": quest_revision,
            "description": "Change only the existing Quest context.",
            "parent_catalog_id": "g1r:quest-parent:swampcamp_scchapter2",
            "giver_catalog_id": "g1r:npc:om_grd_asghan_263"
        }))
        .unwrap();
        request.to_canonical_json().unwrap()
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
    fn exact_raw_wire_rejects_duplicate_unknown_missing_and_wrongly_typed_fields() {
        let valid = raw_request(valid_shape());
        let parsed: PrepareQuestContextWirePayload = parse_exact_wire(&valid).unwrap();
        assert_eq!(parsed.current_project_json, "{}");

        let cases = [
            format!(
                "{{\"command\":\"{COMMAND}\",\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"game_root\":\"g\",\"quest_context_request_json\":\"{{}}\",\"root\":\"r\"}}}}"
            ),
            format!(
                "{{\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"game_root\":\"g\",\"game_root\":\"forged\",\"quest_context_request_json\":\"{{}}\",\"root\":\"r\"}}}}"
            ),
            raw_request(json!({
                "current_project_json": "{}", "game_root": "g",
                "quest_context_request_json": "{}", "root": "r", "authority": "forged"
            })),
            raw_request(json!({
                "game_root": "g", "quest_context_request_json": "{}", "root": "r"
            })),
            raw_request(json!({
                "current_project_json": {}, "game_root": "g",
                "quest_context_request_json": "{}", "root": "r"
            })),
        ];
        for input in cases {
            assert_eq!(
                prepare_revision3_quest_context_edit_v1_raw(&input)["error"]["code"],
                "AUTHORING_REVISION3_QUEST_CONTEXT_REQUEST_INVALID"
            );
        }
    }

    #[test]
    fn public_dispatch_preserves_duplicate_rejection_for_the_raw_route() {
        let duplicate_payload = format!(
            "{{\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"game_root\":\"g\",\"game_root\":\"forged\",\"quest_context_request_json\":\"{{}}\",\"root\":\"r\"}}}}"
        );
        let response: Value =
            serde_json::from_str(&crate::execute_json(&duplicate_payload)).unwrap();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_QUEST_CONTEXT_REQUEST_INVALID"
        );
    }

    #[test]
    fn nested_transports_and_paths_are_bounded_before_filesystem_access() {
        let mut payload = valid_shape();
        payload["root"] = Value::String("x".repeat(MAX_PATH_BYTES + 1));
        assert_eq!(
            prepare_revision3_quest_context_edit_v1_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_QUEST_CONTEXT_REQUEST_INVALID"
        );

        let mut payload = valid_shape();
        payload["current_project_json"] = Value::String("x".repeat(MAX_PROJECT_JSON_BYTES + 1));
        assert_eq!(
            prepare_revision3_quest_context_edit_v1_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_QUEST_CONTEXT_PROJECT_LIMIT"
        );

        let mut payload = valid_shape();
        payload["quest_context_request_json"] =
            Value::String("x".repeat(MAX_REVISION3_QUEST_CONTEXT_EDIT_REQUEST_JSON_BYTES_V1 + 1));
        assert_eq!(
            prepare_revision3_quest_context_edit_v1_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_QUEST_CONTEXT_REQUEST_LIMIT"
        );
    }

    #[test]
    fn invalid_or_unavailable_inputs_never_change_the_published_store_head() {
        let (temp, project_json, fixed_head) = published_store_at_revision(7);
        let project = ProjectRevision3::from_json(&project_json).unwrap();
        let head: WorkingHead = serde_json::from_slice(&fixed_head).unwrap();
        let before = snapshot_regular_files(temp.path());
        let response = prepare_revision3_quest_context_edit_v1_raw(&raw_request(json!({
            "current_project_json": project_json,
            "game_root": temp.path().join("missing-game"),
            "quest_context_request_json": request_json(&project, &head, 0),
            "root": temp.path(),
        })));
        assert!(!response["ok"].as_bool().unwrap_or(false));
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            fixed_head
        );
        assert_eq!(snapshot_regular_files(temp.path()), before);
        let encoded = response.to_string();
        assert!(!encoded.contains(temp.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn basis_revision_is_signed_wire_safe_and_rejected_before_any_candidate_write() {
        assert!(validate_basis_revision(MAX_BASIS_REVISION).is_ok());
        assert_eq!(
            validate_basis_revision(MAX_BASIS_REVISION + 1)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_QUEST_CONTEXT_REVISION_LIMIT"
        );

        for (revision, must_hit_revision_limit) in
            [(MAX_BASIS_REVISION, false), (MAX_BASIS_REVISION + 1, true)]
        {
            let (temp, project_json, fixed_head) = published_store_at_revision(revision);
            let project = ProjectRevision3::from_json(&project_json).unwrap();
            let head: WorkingHead = serde_json::from_slice(&fixed_head).unwrap();
            let before = snapshot_regular_files(temp.path());
            let response = prepare_revision3_quest_context_edit_v1_raw(&raw_request(json!({
                "current_project_json": project_json,
                "game_root": temp.path().join("missing-game"),
                "quest_context_request_json": request_json(&project, &head, 0),
                "root": temp.path(),
            })));
            if must_hit_revision_limit {
                assert_eq!(
                    response["error"]["code"],
                    "AUTHORING_REVISION3_QUEST_CONTEXT_REVISION_LIMIT"
                );
            } else {
                assert_ne!(
                    response["error"]["code"],
                    "AUTHORING_REVISION3_QUEST_CONTEXT_REVISION_LIMIT"
                );
            }
            assert_eq!(
                fs::read(temp.path().join("gore-project.json")).unwrap(),
                fixed_head
            );
            assert_eq!(snapshot_regular_files(temp.path()), before);
        }
    }

    #[test]
    fn response_contract_exposes_only_prepare_only_statuses_and_friendly_ids() {
        let head = WorkingHead {
            store_format: gore_authoring::WorkingStoreFormat,
            snapshot: gore_authoring::ContentSeal {
                byte_len: 1,
                sha256: gore_authoring::Sha256Digest::from_bytes([7; 32]),
            },
        };
        assert!(canonical_head_json(&head).unwrap().contains("snapshot"));
        let response = json!({
            "ok": true,
            "outcome": "prepared_unpublished",
            "story_catalog_seal": {
                "byte_len": 1,
                "sha256": "1111111111111111111111111111111111111111111111111111111111111111"
            },
            "parent_catalog_id": "g1r:quest-parent:swampcamp_scchapter2",
            "giver_catalog_id": "g1r:npc:om_grd_asghan_263",
            "build_status": "blocked",
            "runtime_status": "runtime_unqualified",
            "publication_status": "not_supported",
        });
        enforce_response_budget(&response).unwrap();
        assert_ne!(response["publication_status"], "published");
        assert!(response.get("compile_ready").is_none());
        assert!(response.get("runtime_qualified").is_none());
        assert!(response.get("artifact_json").is_none());
        assert!(response.get("runtime_class").is_none());
        assert!(response.get("runtime_unique_name").is_none());
    }

    #[test]
    fn stable_binding_and_semantic_conflicts_keep_distinct_wire_codes() {
        use gore_story_inventory::Revision3QuestContextEditErrorV1 as E;

        assert_eq!(
            map_transaction_error(E::Binding(
                Revision3QuestContextEditBindingErrorV1::CurrentHeadMismatch,
            ))
            .code,
            "AUTHORING_REVISION3_QUEST_CONTEXT_HEAD_CONFLICT"
        );
        assert_eq!(
            map_transaction_error(E::Binding(
                Revision3QuestContextEditBindingErrorV1::ProjectRevisionMismatch,
            ))
            .code,
            "AUTHORING_REVISION3_QUEST_CONTEXT_PROJECT_CONFLICT"
        );
        assert_eq!(
            map_transaction_error(E::Binding(
                Revision3QuestContextEditBindingErrorV1::ProjectTargetMismatch,
            ))
            .code,
            "AUTHORING_REVISION3_QUEST_CONTEXT_TARGET_CONFLICT"
        );
        assert_eq!(
            map_transaction_error(E::Binding(
                Revision3QuestContextEditBindingErrorV1::StoryCatalogSealMismatch,
            ))
            .code,
            "AUTHORING_REVISION3_QUEST_CONTEXT_CATALOG_CONFLICT"
        );
        assert_eq!(
            map_transaction_conflict(Revision3QuestContextEditConflictV1::NoChanges).code,
            "AUTHORING_REVISION3_QUEST_CONTEXT_NO_CHANGES"
        );
        assert_eq!(
            map_transaction_conflict(Revision3QuestContextEditConflictV1::ProjectRevisionOverflow,)
                .code,
            "AUTHORING_REVISION3_QUEST_CONTEXT_REVISION_LIMIT"
        );
        assert_eq!(
            map_capability_error(
                Revision3QuestCollisionCapabilityErrorV2::PriorQuestParentDrift {
                    quest: gore_authoring::EntityId::from_bytes([9; 16]),
                },
            )
            .code,
            "AUTHORING_REVISION3_QUEST_CONTEXT_CATALOG_CONFLICT"
        );
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
            "AUTHORING_REVISION3_QUEST_CONTEXT_STORE_GAME_ALIAS"
        );

        let nested_store_root = game_root.join("project");
        fs::create_dir(&nested_store_root).unwrap();
        let nested_store =
            WorkingProjectStore::open_existing(&nested_store_root, ffi_store_limits()).unwrap();
        assert_eq!(
            ensure_store_is_outside_game(&nested_store, &game_root)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_QUEST_CONTEXT_STORE_GAME_ALIAS"
        );

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
            "AUTHORING_REVISION3_QUEST_CONTEXT_STORE_GAME_ALIAS"
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
    fn fixed_basis_helper_rejects_an_external_head_change() {
        let (temp, project_json, fixed_head) = published_store_at_revision(7);
        let store = WorkingProjectStore::open_existing(temp.path(), ffi_store_limits()).unwrap();
        let expected_project = ProjectRevision3::from_json(&project_json).unwrap();
        let expected_head: WorkingHead = serde_json::from_slice(&fixed_head).unwrap();
        require_fixed_basis(&store, &expected_head, &expected_project).unwrap();

        let replacement_json = project_json_at_revision(8);
        let replacement = ProjectRevision3::from_json(&replacement_json).unwrap();
        let prepared = store
            .prepare_revision3_checkpoint(Some(&expected_head), &replacement)
            .unwrap();
        fs::write(temp.path().join("gore-project.json"), prepared.head_bytes).unwrap();
        assert_eq!(
            require_fixed_basis(&store, &expected_head, &expected_project)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_QUEST_CONTEXT_HEAD_CONFLICT"
        );
    }

    #[test]
    #[ignore = "requires GORE_STORY_GAME_ROOT pointing at the pinned supported game generation"]
    fn live_native_path_prepares_context_edit_and_never_publishes_the_candidate() {
        let game_root = std::env::var("GORE_STORY_GAME_ROOT")
            .expect("set GORE_STORY_GAME_ROOT to run the live Quest context FFI test");
        let (catalog, _, _) = build_fresh_game_inputs(Path::new(&game_root)).unwrap();
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let base_json = live_project_json(&catalog.generation().executable);
        let base = ProjectRevision3::from_json(&base_json).unwrap();
        let published = store.prepare_revision3_checkpoint(None, &base).unwrap();
        fs::write(temp.path().join("gore-project.json"), &published.head_bytes).unwrap();

        let draft_input = json!({
            "command": "authoring_store_prepare_revision3_quest_draft_v3",
            "payload": {
                "current_project_json": base_json,
                "game_root": game_root,
                "quest_request_json": live_draft_request_json(&base, &published.head),
                "root": temp.path(),
            }
        })
        .to_string();
        let draft = crate::authoring_story_quest_revision3::prepare_revision3_quest_draft_v3_raw(
            &draft_input,
        );
        assert_eq!(draft["ok"], true, "{draft}");
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            published.head_bytes
        );

        let draft_head_json = draft["head_json"].as_str().unwrap();
        fs::write(temp.path().join("gore-project.json"), draft_head_json).unwrap();
        let draft_project_json = draft["project_json"].as_str().unwrap();
        let draft_project = ProjectRevision3::from_json(draft_project_json).unwrap();
        let draft_head: WorkingHead = serde_json::from_str(draft_head_json).unwrap();
        let context_request: Revision3QuestContextEditRequestV1 = serde_json::from_value(json!({
            "expected_head": draft_head,
            "expected_project_id": draft_project.project_id,
            "expected_revision": draft_project.revision,
            "expected_story_catalog_seal": catalog.catalog_seal(),
            "quest_id": "80808080808080808080808080808080",
            "expected_quest_revision": 0,
            "description": "The edited exact Quest context.",
            "parent_catalog_id": "g1r:quest-parent:swampcamp_scchapter2",
            "giver_catalog_id": "g1r:npc:om_grd_asghan_263"
        }))
        .unwrap();
        let fixed_draft_head = fs::read(temp.path().join("gore-project.json")).unwrap();
        let response = prepare_revision3_quest_context_edit_v1_raw(&raw_request(json!({
            "current_project_json": draft_project_json,
            "game_root": std::env::var("GORE_STORY_GAME_ROOT").unwrap(),
            "quest_context_request_json": context_request.to_canonical_json().unwrap(),
            "root": temp.path(),
        })));
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["outcome"], "prepared_unpublished");
        assert_eq!(response["revision"], 2);
        assert_eq!(response["quest_revision"], 1);
        assert_eq!(response["module_revision"], 1);
        assert_eq!(
            response["story_catalog_seal"],
            json!(catalog.catalog_seal())
        );
        assert_eq!(response["build_status"], "blocked");
        assert_eq!(response["runtime_status"], "runtime_unqualified");
        assert_eq!(response["publication_status"], "not_supported");
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            fixed_draft_head
        );

        let reopened = store
            .open_revision3_head_bytes(
                response["head_json"].as_str().unwrap().as_bytes(),
                AssetVerification::Full,
            )
            .unwrap();
        assert_eq!(
            reopened.project.to_canonical_json().unwrap(),
            response["project_json"].as_str().unwrap()
        );
        let quest_id = gore_authoring::EntityId::from_bytes([0x80; 16]);
        let Revision3EntityPayload::QuestDraft(edited) =
            &reopened.project.entities[&quest_id].payload
        else {
            panic!("expected edited Quest")
        };
        assert_eq!(edited.input.description, "The edited exact Quest context.");
        assert_eq!(reopened.project.asset_store, draft_project.asset_store);
        let encoded = response.to_string();
        assert!(!encoded.contains(temp.path().to_string_lossy().as_ref()));
        assert!(!encoded.contains(&std::env::var("GORE_STORY_GAME_ROOT").unwrap()));
        assert!(response.get("artifact_json").is_none());
    }
}
