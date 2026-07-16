//! Native, prepare-only orchestration for one existing revision-3 NPC profile edit.
//!
//! The route rebuilds the pinned Story catalog and broad NPC archetype catalog from fresh native
//! inputs, resolves both the exact current and desired parent triples, and consumes that context in
//! the filesystem-free profile transaction. It fully reopens one immutable candidate checkpoint
//! but never publishes the fixed project head, builds collision authority, touches a game/save, or
//! grants compiler, deployment, spawn, runtime, catalog, or publication authority.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use gore_authoring::{
    apply_revision3_npc_profile_edit_transaction_v1, AssetVerification,
    ContentSeal as AuthoringContentSeal, GameGenerationAnchor, Revision2NpcParentClassInput,
    Revision3EntityKind, Revision3EntityPayload, Revision3NpcCatalogSelectionV1,
    Revision3NpcProfileCatalogContextV1, Revision3NpcProfileEditBuildStatusV1,
    Revision3NpcProfileEditCatalogAuthorityV1, Revision3NpcProfileEditCollisionAuthorityV1,
    Revision3NpcProfileEditConflictV1, Revision3NpcProfileEditErrorV1,
    Revision3NpcProfileEditEvaluationV1, Revision3NpcProfileEditPublicationStatusV1,
    Revision3NpcProfileEditRequestV1, Revision3NpcProfileEditRuntimeStatusV1,
    Sha256Digest as AuthoringSha256Digest, WorkingHead, WorkingProjectStore, WorkingStoreError,
    WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
    MAX_REVISION3_NPC_PROFILE_EDIT_REQUEST_JSON_BYTES_V1,
};
use gore_npc_catalog::{build_npc_archetype_catalog, NpcArchetypeCatalogFile, NpcCatalogError};
use gore_story_catalog::{
    build_known_catalog_with_shipping_snapshot, AuthoringClassSelection, AuthoringNpcSelection,
    CatalogError, ContentSeal, GameGenerationSeal, GenerationInputLimits, StoryCatalogFile,
};
use gore_story_inventory::MAX_BINDS_CACHE_SOURCE_BYTES;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

use crate::authoring_story_inventory::{read_source_no_follow, SourceReadError};
use crate::err;

pub(super) const COMMAND: &str = "authoring_store_prepare_revision3_npc_profile_edit_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_BASIS_REVISION: u64 = i64::MAX as u64 - 1;
const MAX_WIRE_BYTES: usize = MAX_PROJECT_JSON_BYTES * 2
    + MAX_REVISION3_NPC_PROFILE_EDIT_REQUEST_JSON_BYTES_V1 * 2
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
struct PrepareNpcProfileWirePayload {
    current_project_json: String,
    game_root: String,
    npc_profile_request_json: String,
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

struct FreshNpcProfileInputs {
    story_catalog: StoryCatalogFile,
    npc_catalog: NpcArchetypeCatalogFile,
    shipping: Vec<u8>,
}

pub(super) fn prepare_revision3_npc_profile_edit_v1_raw(input: &str) -> Value {
    prepare_revision3_npc_profile_edit_v1_inner(input).unwrap_or_else(Failure::response)
}

fn prepare_revision3_npc_profile_edit_v1_inner(input: &str) -> Result<Value, Failure> {
    prepare_revision3_npc_profile_edit_v1_inner_with_test_seams(input, || {}, || {})
}

fn prepare_revision3_npc_profile_edit_v1_inner_with_test_seams<B, F>(
    input: &str,
    before_checkpoint: B,
    final_guard: F,
) -> Result<Value, Failure>
where
    B: FnOnce(),
    F: FnOnce(),
{
    let payload: PrepareNpcProfileWirePayload = parse_exact_wire(input)?;
    validate_payload(&payload)?;
    let request = Revision3NpcProfileEditRequestV1::from_json(&payload.npc_profile_request_json)
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
            "AUTHORING_REVISION3_NPC_PROFILE_STORE_INVARIANT",
            "the exact current revision-3 project could not be serialized canonically",
        )
    })?;
    if canonical_basis.as_bytes() != payload.current_project_json.as_bytes() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_PROJECT_CONFLICT",
            "current_project_json differs from the exact published revision-3 project",
        ));
    }
    bind_request_to_basis(&basis.head, &basis.project, &request)?;

    let game_root = PathBuf::from(&payload.game_root);
    ensure_store_is_outside_game(&store, &game_root)?;
    let fresh = build_fresh_game_inputs(&game_root)?;
    let FreshNpcProfileInputs {
        story_catalog,
        npc_catalog,
        shipping,
    } = fresh;
    let story_catalog_seal = authoring_seal(story_catalog.catalog_seal());
    let npc_catalog_seal = authoring_seal(npc_catalog.catalog_seal());
    if request.expected_story_catalog_seal != story_catalog_seal
        || request.expected_npc_catalog_seal != npc_catalog_seal
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_CATALOG_CONFLICT",
            "the trusted Story or NPC catalog changed after the profile choices were shown",
        ));
    }
    let current_selection = resolve_native_selection(
        &story_catalog,
        &npc_catalog,
        &request.expected_parent_catalog_id,
    )?;
    let desired_selection =
        resolve_native_selection(&story_catalog, &npc_catalog, &request.parent_catalog_id)?;

    // Close both mutable authority windows immediately before consuming the fresh context.
    require_fixed_basis(&store, &basis.head, &basis.project)?;
    revalidate_game_inputs(&story_catalog, &game_root, &shipping)?;
    ensure_store_is_outside_game(&store, &game_root)?;
    let outcome = match apply_revision3_npc_profile_edit_transaction_v1(
        &basis.head,
        &payload.current_project_json,
        &payload.npc_profile_request_json,
        Revision3NpcProfileCatalogContextV1 {
            current_selection,
            desired_selection,
        },
    )
    .map_err(map_transaction_error)?
    {
        Revision3NpcProfileEditEvaluationV1::Applied(outcome) => *outcome,
        Revision3NpcProfileEditEvaluationV1::Rejected(rejection) => {
            return Err(map_transaction_conflict(rejection.conflict));
        }
    };
    require_signed_serializable(&outcome.project)?;
    validate_outcome_binding(&basis, &request, &outcome)?;

    require_fixed_basis(&store, &basis.head, &basis.project)?;
    revalidate_game_inputs(&story_catalog, &game_root, &shipping)?;
    ensure_store_is_outside_game(&store, &game_root)?;
    before_checkpoint();
    let prepared = store
        .prepare_revision3_checkpoint(Some(&basis.head), &outcome.project)
        .map_err(map_store_error)?;
    let reopened = store
        .open_revision3_head_bytes(&prepared.head_bytes, AssetVerification::Full)
        .map_err(map_store_error)?;
    if reopened.head != prepared.head || reopened.project != outcome.project {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_STORE_INVARIANT",
            "the prepared NPC profile checkpoint did not fully reopen exactly",
        ));
    }
    let reopened_json = reopened.project.to_canonical_json().map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_STORE_INVARIANT",
            "the reopened NPC profile candidate could not be serialized",
        )
    })?;
    if reopened_json != outcome.canonical_project_json {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_STORE_INVARIANT",
            "the reopened NPC profile candidate changed canonical bytes",
        ));
    }

    final_guard();
    require_fixed_basis(&store, &basis.head, &basis.project)?;
    revalidate_game_inputs(&story_catalog, &game_root, &shipping)?;
    ensure_store_is_outside_game(&store, &game_root)?;

    let basis_head_json = canonical_head_json(&basis.head)?;
    let candidate_head_json = String::from_utf8(prepared.head_bytes).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_STORE_INVARIANT",
            "the prepared NPC profile head is not UTF-8 JSON",
        )
    })?;
    if candidate_head_json.is_empty() || candidate_head_json.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_RESPONSE_LIMIT",
            "the prepared NPC profile head exceeds its bounded transport limit",
        ));
    }
    require_signed_serializable(&prepared.head)?;
    let build_status = match outcome.build_status {
        Revision3NpcProfileEditBuildStatusV1::Blocked => "blocked",
    };
    let runtime_status = match outcome.runtime_status {
        Revision3NpcProfileEditRuntimeStatusV1::RuntimeUnqualified => "runtime_unqualified",
    };
    let catalog_authority = match outcome.catalog_authority {
        Revision3NpcProfileEditCatalogAuthorityV1::NotGranted => "not_granted",
    };
    let collision_authority = match outcome.collision_authority {
        Revision3NpcProfileEditCollisionAuthorityV1::NotGranted => "not_granted",
    };
    let publication_status = match outcome.publication_status {
        Revision3NpcProfileEditPublicationStatusV1::NotSupported => "not_supported",
    };
    let response = json!({
        "ok": true,
        "outcome": "prepared_unpublished",
        "basis_head_json": basis_head_json,
        "head_json": candidate_head_json,
        "project_json": outcome.canonical_project_json,
        "project_id": outcome.project.project_id.to_string(),
        "revision": outcome.project.revision,
        "npc_id": outcome.npc_id.to_string(),
        "npc_revision": outcome.npc_revision,
        "script_module_id": outcome.script_module_id.to_string(),
        "script_module_revision": outcome.script_module_revision,
        "display_name": request.display_name,
        "previous_parent_catalog_id": request.expected_parent_catalog_id,
        "parent_catalog_id": request.parent_catalog_id,
        "story_catalog_seal": story_catalog_seal,
        "npc_catalog_seal": npc_catalog_seal,
        "name_changed": outcome.name_changed,
        "archetype_changed": outcome.archetype_changed,
        "module_regenerated": outcome.module_regenerated,
        "build_status": build_status,
        "runtime_status": runtime_status,
        "catalog_authority": catalog_authority,
        "collision_authority": collision_authority,
        "publication_status": publication_status,
    });
    enforce_response_budget(&response)?;

    require_fixed_basis(&store, &basis.head, &basis.project)?;
    revalidate_game_inputs(&story_catalog, &game_root, &shipping)?;
    ensure_store_is_outside_game(&store, &game_root)?;
    Ok(response)
}

fn parse_exact_wire<P>(input: &str) -> Result<P, Failure>
where
    P: DeserializeOwned + Serialize,
{
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_INPUT_LIMIT",
            format!("NPC profile request exceeds the {MAX_WIRE_BYTES}-byte wire limit"),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    let canonical = serde_json::to_string(&request).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_INVARIANT",
            "the NPC profile outer request could not be serialized",
        )
    })?;
    if canonical != input {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_payload(payload: &PrepareNpcProfileWirePayload) -> Result<(), Failure> {
    validate_path(&payload.root)?;
    validate_path(&payload.game_root)?;
    if payload.current_project_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_PROJECT_LIMIT",
            format!("current_project_json exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ));
    }
    if payload.npc_profile_request_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.npc_profile_request_json.len() > MAX_REVISION3_NPC_PROFILE_EDIT_REQUEST_JSON_BYTES_V1
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_REQUEST_LIMIT",
            format!(
                "npc_profile_request_json exceeds the {MAX_REVISION3_NPC_PROFILE_EDIT_REQUEST_JSON_BYTES_V1}-byte limit"
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
    request: &Revision3NpcProfileEditRequestV1,
) -> Result<(), Failure> {
    if request.expected_head != *head {
        return Err(head_conflict());
    }
    if request.expected_project_id != project.project_id
        || request.expected_revision != project.revision
        || request.expected_target != project.target
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_PROJECT_CONFLICT",
            "the NPC profile request project differs from the exact published project",
        ));
    }
    let Some(npc_entity) = project.entities.get(&request.npc_id) else {
        return Err(npc_conflict(
            "the requested NPC does not exist in the exact published project",
        ));
    };
    let Revision3EntityPayload::NpcDraft(npc) = &npc_entity.payload else {
        return Err(npc_conflict(
            "the requested entity is not an NPC in the exact published project",
        ));
    };
    if npc_entity.revision != request.expected_npc_revision {
        return Err(npc_conflict(
            "the requested NPC revision differs from the exact published entity revision",
        ));
    }
    if request.npc_id == request.script_module_id
        || npc.script_module.project_id != project.project_id
        || npc.script_module.id != request.script_module_id
        || npc.script_module.expected_kind != Revision3EntityKind::ScriptModule
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_PROJECT_INVALID",
            "the exact NPC does not have the requested closed local module binding",
        ));
    }
    let Some(module_entity) = project.entities.get(&request.script_module_id) else {
        return Err(module_conflict(
            "the requested NPC module does not exist in the exact published project",
        ));
    };
    if !matches!(
        module_entity.payload,
        Revision3EntityPayload::ScriptModule(_)
    ) {
        return Err(module_conflict(
            "the requested NPC module changed kind in the exact published project",
        ));
    }
    if module_entity.revision != request.expected_script_module_revision {
        return Err(module_conflict(
            "the requested NPC module revision differs from the exact published entity revision",
        ));
    }
    Ok(())
}

fn validate_outcome_binding(
    basis: &gore_authoring::OpenedRevision3Checkpoint,
    request: &Revision3NpcProfileEditRequestV1,
    outcome: &gore_authoring::Revision3NpcProfileEditOutcomeV1,
) -> Result<(), Failure> {
    let expected_npc_revision = request.expected_npc_revision.checked_add(1);
    let expected_module_revision = request
        .expected_script_module_revision
        .checked_add(u64::from(outcome.archetype_changed));
    let expected_name_changed = basis
        .project
        .entities
        .get(&request.npc_id)
        .is_some_and(|entity| entity.display_name != request.display_name);
    if outcome.basis_head != basis.head
        || outcome.project.project_id != basis.project.project_id
        || outcome.project.target != basis.project.target
        || outcome.project.revision != basis.project.revision + 1
        || outcome.npc_id != request.npc_id
        || outcome.script_module_id != request.script_module_id
        || Some(outcome.npc_revision) != expected_npc_revision
        || Some(outcome.script_module_revision) != expected_module_revision
        || outcome.name_changed != expected_name_changed
        || outcome.module_regenerated != outcome.archetype_changed
        || (!outcome.name_changed && !outcome.archetype_changed)
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_INVARIANT",
            "the NPC profile transaction changed its exact project/request binding",
        ));
    }
    Ok(())
}

fn build_fresh_game_inputs(game_root: &Path) -> Result<FreshNpcProfileInputs, Failure> {
    let g1r = resolve_g1r_root(game_root);
    let executable = g1r
        .join("Binaries")
        .join("Win64")
        .join("G1R-Win64-Shipping.exe");
    let binds_path = g1r.join("Script").join("Binds.Cache");
    let shipping = gore_mod::pristine_script_cache(game_root).map_err(map_pristine_error)?;
    let story_catalog = build_known_catalog_with_shipping_snapshot(
        &executable,
        &shipping,
        &binds_path,
        GenerationInputLimits::default(),
    )
    .map_err(map_catalog_error)?;
    story_catalog
        .revalidate_generation_inputs()
        .map_err(map_catalog_error)?;
    let binds = read_source_no_follow(&binds_path, MAX_BINDS_CACHE_SOURCE_BYTES as u64)
        .map_err(map_source_read_error)?;
    if binds.len() as u64 != story_catalog.generation().binds_cache.byte_len
        || Sha256::digest(&binds).as_slice()
            != story_catalog.generation().binds_cache.sha256.as_bytes()
    {
        return Err(input_changed());
    }
    let npc_catalog = build_npc_archetype_catalog(&story_catalog, &shipping, &binds)
        .map_err(map_npc_catalog_error)?;
    story_catalog
        .revalidate_generation_inputs()
        .map_err(map_catalog_error)?;
    Ok(FreshNpcProfileInputs {
        story_catalog,
        npc_catalog,
        shipping,
    })
}

fn resolve_native_selection(
    story_catalog: &StoryCatalogFile,
    npc_catalog: &NpcArchetypeCatalogFile,
    catalog_id: &str,
) -> Result<Revision3NpcCatalogSelectionV1, Failure> {
    if npc_catalog.generation() != story_catalog.generation()
        || npc_catalog.story_catalog_seal() != story_catalog.catalog_seal()
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_CATALOG_INVARIANT",
            "the native NPC archetype catalog is not bound to the exact Story catalog",
        ));
    }
    let selections = story_catalog
        .authoring_selections()
        .map_err(map_catalog_error)?;
    if selections.generation != *story_catalog.generation()
        || selections.catalog_seal != *story_catalog.catalog_seal()
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_CATALOG_INVARIANT",
            "the Story authoring projection changed its exact catalog binding",
        ));
    }
    let mut matching_selections = selections
        .npcs
        .iter()
        .filter(|candidate| candidate.catalog_id == catalog_id);
    let selected = matching_selections.next().ok_or_else(|| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_CATALOG_SELECTION_INVALID",
            "the requested NPC parent catalog ID is unavailable",
        )
    })?;
    if matching_selections.next().is_some() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_CATALOG_SELECTION_INVALID",
            "the requested NPC parent catalog ID is ambiguous",
        ));
    }
    require_offline_qualified_selection(selected)?;
    let mut matching_records = npc_catalog.records().iter().filter(|record| {
        record.spawn.class_name == selected.spawn_definition.runtime_class
            && record.spawn.source_seal == selected.spawn_definition.source_seal
            && record.ai_config.class_name == selected.ai_agent_config.runtime_class
            && record.ai_config.source_seal == selected.ai_agent_config.source_seal
            && record.character_definition.class_name == selected.character_definition.runtime_class
            && record.character_definition.source_seal == selected.character_definition.source_seal
    });
    if matching_records.next().is_none() || matching_records.next().is_some() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_CATALOG_SELECTION_INVALID",
            "the selected Story NPC does not have one exact archetype linkage record",
        ));
    }

    let target = authoring_target(story_catalog.generation());
    Ok(Revision3NpcCatalogSelectionV1 {
        generation: target.clone(),
        catalog_id: selected.catalog_id.clone(),
        story_catalog_seal: authoring_seal(story_catalog.catalog_seal()),
        npc_catalog_seal: authoring_seal(npc_catalog.catalog_seal()),
        parent_character_definition: authoring_parent(&target, &selected.character_definition),
        parent_ai_agent_config: authoring_parent(&target, &selected.ai_agent_config),
        parent_spawn_definition: authoring_parent(&target, &selected.spawn_definition),
    })
}

fn require_offline_qualified_selection(selected: &AuthoringNpcSelection) -> Result<(), Failure> {
    if selected.discovery_status != "sealed_cache_defaults_verified"
        || selected.authoring_qualification != "offline_qualified"
        || selected.runtime_qualification != "runtime_unqualified"
        || !selected.blocks_build
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_CATALOG_SELECTION_UNQUALIFIED",
            "the requested NPC parent is not qualified for offline clone authoring",
        ));
    }
    Ok(())
}

fn authoring_parent(
    target: &GameGenerationAnchor,
    value: &AuthoringClassSelection,
) -> Revision2NpcParentClassInput {
    Revision2NpcParentClassInput {
        generation: target.clone(),
        source_seal: authoring_seal(&value.source_seal),
        catalog_layer: value.catalog_layer.clone(),
        canonical_selector: value.authoring_selector.clone(),
        runtime_class: value.runtime_class.clone(),
    }
}

fn authoring_target(generation: &GameGenerationSeal) -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: authoring_seal(&generation.executable),
    }
}

fn authoring_seal(value: &ContentSeal) -> AuthoringContentSeal {
    AuthoringContentSeal {
        byte_len: value.byte_len,
        sha256: AuthoringSha256Digest::from_bytes(*value.sha256.as_bytes()),
    }
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
            "AUTHORING_REVISION3_NPC_PROFILE_STORE_PATH_UNSAFE",
            "the revision-3 working-store root could not be resolved safely",
        )
    })?;
    let install_root = fs::canonicalize(semantic_install_root(game_root)).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_INPUT_UNAVAILABLE",
            "the selected game installation root could not be resolved safely",
        )
    })?;
    if store_root.starts_with(&install_root) || install_root.starts_with(&store_root) {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_STORE_GAME_ALIAS",
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
            "AUTHORING_REVISION3_NPC_PROFILE_REVISION_LIMIT",
            format!("the published basis revision exceeds {MAX_BASIS_REVISION}"),
        ));
    }
    Ok(())
}

fn require_signed_serializable(value: &impl Serialize) -> Result<(), Failure> {
    let value = serde_json::to_value(value).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_INVARIANT",
            "an NPC profile wire value could not be inspected",
        )
    })?;
    require_signed_json_value(&value)
}

fn require_signed_json_value(value: &Value) -> Result<(), Failure> {
    match value {
        Value::Number(number) if number.as_u64().is_some_and(|value| value > i64::MAX as u64) => {
            Err(Failure::new(
                "AUTHORING_REVISION3_NPC_PROFILE_SIGNED_WIRE_LIMIT",
                "an NPC profile wire integer exceeds the signed 64-bit transport range",
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
            "AUTHORING_REVISION3_NPC_PROFILE_INVARIANT",
            "the NPC profile basis head could not be serialized",
        )
    })?;
    if value.is_empty() || value.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_RESPONSE_LIMIT",
            "the NPC profile basis head exceeds its bounded transport limit",
        ));
    }
    Ok(value)
}

fn enforce_response_budget(response: &Value) -> Result<(), Failure> {
    let encoded = serde_json::to_vec(response).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_INVARIANT",
            "the NPC profile response could not be serialized",
        )
    })?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_RESPONSE_LIMIT",
            "the NPC profile response exceeds its bounded transport budget",
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
        "AUTHORING_REVISION3_NPC_PROFILE_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and exactly current_project_json, game_root, npc_profile_request_json, and root",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_NPC_PROFILE_HEAD_CONFLICT",
        "the published revision-3 head changed or differs from the NPC profile request",
    )
}

fn npc_conflict(message: &'static str) -> Failure {
    Failure::new("AUTHORING_REVISION3_NPC_PROFILE_NPC_CONFLICT", message)
}

fn module_conflict(message: &'static str) -> Failure {
    Failure::new("AUTHORING_REVISION3_NPC_PROFILE_MODULE_CONFLICT", message)
}

fn input_changed() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_NPC_PROFILE_INPUT_CHANGED",
        "the native game generation changed during NPC profile preparation",
    )
}

fn map_request_error(error: impl std::fmt::Display) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_NPC_PROFILE_REQUEST_INVALID",
        format!("the exact NPC profile request is invalid: {error}"),
    )
}

fn map_transaction_error(error: Revision3NpcProfileEditErrorV1) -> Failure {
    match error {
        Revision3NpcProfileEditErrorV1::InvalidProject(_) => Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_PROJECT_INVALID",
            "the exact current revision-3 project is invalid",
        ),
        Revision3NpcProfileEditErrorV1::InvalidRequest(error) => map_request_error(error),
        Revision3NpcProfileEditErrorV1::ReopenCandidate(_)
        | Revision3NpcProfileEditErrorV1::CanonicalReopenMismatch
        | Revision3NpcProfileEditErrorV1::CandidatePreservationMismatch => Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_INVARIANT",
            "the NPC profile candidate failed its exact preservation/reopen checks",
        ),
    }
}

fn map_transaction_conflict(error: Revision3NpcProfileEditConflictV1) -> Failure {
    let code = match &error {
        Revision3NpcProfileEditConflictV1::CurrentHeadMismatch => {
            "AUTHORING_REVISION3_NPC_PROFILE_HEAD_CONFLICT"
        }
        Revision3NpcProfileEditConflictV1::ProjectIdentityMismatch { .. }
        | Revision3NpcProfileEditConflictV1::ProjectRevisionConflict { .. }
        | Revision3NpcProfileEditConflictV1::ProjectTargetMismatch => {
            "AUTHORING_REVISION3_NPC_PROFILE_PROJECT_CONFLICT"
        }
        Revision3NpcProfileEditConflictV1::NpcRevisionConflict { .. } => {
            "AUTHORING_REVISION3_NPC_PROFILE_NPC_CONFLICT"
        }
        Revision3NpcProfileEditConflictV1::ScriptModuleRevisionConflict { .. } => {
            "AUTHORING_REVISION3_NPC_PROFILE_MODULE_CONFLICT"
        }
        Revision3NpcProfileEditConflictV1::StoryCatalogSealMismatch
        | Revision3NpcProfileEditConflictV1::NpcCatalogSealMismatch => {
            "AUTHORING_REVISION3_NPC_PROFILE_CATALOG_CONFLICT"
        }
        Revision3NpcProfileEditConflictV1::CurrentCatalogSelectionMismatch
        | Revision3NpcProfileEditConflictV1::DesiredCatalogSelectionMismatch
        | Revision3NpcProfileEditConflictV1::CatalogGenerationMismatch
        | Revision3NpcProfileEditConflictV1::InvalidCatalogContext
        | Revision3NpcProfileEditConflictV1::InvalidDesiredArchetype { .. } => {
            "AUTHORING_REVISION3_NPC_PROFILE_CATALOG_SELECTION_INVALID"
        }
        Revision3NpcProfileEditConflictV1::NoChanges => {
            "AUTHORING_REVISION3_NPC_PROFILE_NO_CHANGES"
        }
        Revision3NpcProfileEditConflictV1::ZeroNpcId
        | Revision3NpcProfileEditConflictV1::ZeroScriptModuleId
        | Revision3NpcProfileEditConflictV1::IdentityCollision
        | Revision3NpcProfileEditConflictV1::InvalidDisplayName => {
            "AUTHORING_REVISION3_NPC_PROFILE_REQUEST_INVALID"
        }
        Revision3NpcProfileEditConflictV1::InvalidNpcEntity { .. }
        | Revision3NpcProfileEditConflictV1::NpcModuleBindingMismatch { .. }
        | Revision3NpcProfileEditConflictV1::InvalidScriptModuleEntity { .. }
        | Revision3NpcProfileEditConflictV1::InvalidNpcClosure { .. }
        | Revision3NpcProfileEditConflictV1::OwnedModuleDrift { .. }
        | Revision3NpcProfileEditConflictV1::StoredArchetypeMismatch
        | Revision3NpcProfileEditConflictV1::CandidateNotPersistable { .. } => {
            "AUTHORING_REVISION3_NPC_PROFILE_PROJECT_INVALID"
        }
        Revision3NpcProfileEditConflictV1::ProjectRevisionOverflow
        | Revision3NpcProfileEditConflictV1::NpcRevisionOverflow { .. }
        | Revision3NpcProfileEditConflictV1::ScriptModuleRevisionOverflow { .. }
        | Revision3NpcProfileEditConflictV1::CandidateTooLarge { .. } => {
            "AUTHORING_REVISION3_NPC_PROFILE_LIMIT"
        }
        Revision3NpcProfileEditConflictV1::TechnicalIdentityChanged => {
            "AUTHORING_REVISION3_NPC_PROFILE_INVARIANT"
        }
    };
    Failure::new(code, error.to_string())
}

fn map_pristine_error(error: gore_mod::ModError) -> Failure {
    let message = error.to_string();
    if message.contains("RECOVERY_REQUIRED") {
        return Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_RECOVERY_REQUIRED",
            "an interrupted deployment must be recovered before NPC profile authoring",
        );
    }
    if message.contains("exceeds the") || message.contains("too large") {
        return Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_INPUT_LIMIT",
            "the pristine Shipping cache exceeds its bounded input limit",
        );
    }
    if message.contains("not a regular non-link file") {
        return Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_INPUT_UNSAFE",
            "the pristine Shipping cache is not a safe regular file",
        );
    }
    Failure::new(
        "AUTHORING_REVISION3_NPC_PROFILE_PRISTINE_UNAVAILABLE",
        "the pristine Shipping cache could not be selected safely",
    )
}

fn map_catalog_error(error: CatalogError) -> Failure {
    match error {
        CatalogError::InvalidLimits(_) | CatalogError::LimitExceeded { .. } => Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_INPUT_LIMIT",
            "a native game-generation input exceeds its bounded resource limit",
        ),
        CatalogError::UnsafeInput(_) | CatalogError::OutputAliasesInput { .. } => Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_INPUT_UNSAFE",
            "a native game-generation input is unsafe",
        ),
        CatalogError::IdentityChanged(_) | CatalogError::SourceChanged { .. } => input_changed(),
        CatalogError::UnsupportedGeneration { .. } => Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_UNSUPPORTED_GENERATION",
            "the selected game does not match the supported pinned generation",
        ),
        CatalogError::Io { source, .. } if source.kind() == io::ErrorKind::NotFound => {
            Failure::new(
                "AUTHORING_REVISION3_NPC_PROFILE_INPUT_MISSING",
                "a required native game-generation input does not exist",
            )
        }
        _ => Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_INPUT_UNAVAILABLE",
            "the native game generation could not be verified safely",
        ),
    }
}

fn map_source_read_error(error: SourceReadError) -> Failure {
    match error {
        SourceReadError::Missing => Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_INPUT_MISSING",
            "a required native game-generation input does not exist",
        ),
        SourceReadError::Unsafe => Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_INPUT_UNSAFE",
            "a native game-generation input is unsafe",
        ),
        SourceReadError::Limit => Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_INPUT_LIMIT",
            "a native game-generation input exceeds its bounded resource limit",
        ),
        SourceReadError::Changed => input_changed(),
        SourceReadError::Io => Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_INPUT_UNAVAILABLE",
            "a native game-generation input could not be read safely",
        ),
    }
}

fn map_npc_catalog_error(error: NpcCatalogError) -> Failure {
    match error {
        NpcCatalogError::LimitExceeded { .. } => Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_CATALOG_LIMIT",
            "the native NPC archetype catalog exceeds its resource limit",
        ),
        NpcCatalogError::UnsupportedGeneration => Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_UNSUPPORTED_GENERATION",
            "the selected game does not match the supported pinned generation",
        ),
        NpcCatalogError::GenerationInputMismatch { .. } => input_changed(),
        _ => Failure::new(
            "AUTHORING_REVISION3_NPC_PROFILE_CATALOG_FAILED",
            "the native NPC archetype catalog could not be rebuilt",
        ),
    }
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::InvalidLimits(_) => {
            "AUTHORING_REVISION3_NPC_PROFILE_STORE_LIMITS_INVALID"
        }
        WorkingStoreError::MissingRoot(_) => "AUTHORING_REVISION3_NPC_PROFILE_STORE_ROOT_MISSING",
        WorkingStoreError::UnsafePath { .. } => "AUTHORING_REVISION3_NPC_PROFILE_STORE_PATH_UNSAFE",
        WorkingStoreError::LimitExceeded { .. } => "AUTHORING_REVISION3_NPC_PROFILE_STORE_LIMIT",
        WorkingStoreError::HeadConflict { .. } => "AUTHORING_REVISION3_NPC_PROFILE_HEAD_CONFLICT",
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_NPC_PROFILE_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => {
            "AUTHORING_REVISION3_NPC_PROFILE_STORE_OBJECT_MISSING"
        }
        WorkingStoreError::SealMismatch { .. } => {
            "AUTHORING_REVISION3_NPC_PROFILE_STORE_SEAL_MISMATCH"
        }
        WorkingStoreError::Collision { .. } => "AUTHORING_REVISION3_NPC_PROFILE_STORE_COLLISION",
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_NPC_PROFILE_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_NPC_PROFILE_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_NPC_PROFILE_STORE_IO"
        }
    };
    Failure::new(code, "the revision-3 NPC profile Store operation failed")
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
        AssetStoreIndex, EntityId, FormatV2, ProjectId, ProjectMeta, ProjectRevision3,
        Revision3Entity, Revision3EntityKind, Revision3EntityPayload, Revision3NpcDraft,
        Revision3NpcDraftInput, Revision3NpcDraftInsertRequestV1, Revision3NpcDraftIntentV1,
        Revision3OriginRef, Revision3TypedRef, SchemaRevisionV3, LOGICAL_NPC_CLONE_GENERATOR_ID,
        LOGICAL_NPC_CLONE_GENERATOR_VERSION,
    };
    use tempfile::TempDir;

    use super::*;

    const NPC_BYTE: u8 = 0x71;
    const MODULE_BYTE: u8 = 0x72;
    const CURRENT_PARENT: &str = "g1r:npc:om_grd_asghan_263";
    const DESIRED_PARENT: &str = "g1r:npc:om_stt_viper_302";

    fn raw_request(payload: Value) -> String {
        json!({"command": COMMAND, "payload": payload}).to_string()
    }

    fn valid_shape() -> Value {
        json!({
            "current_project_json": "{}",
            "game_root": "C:/missing-game",
            "npc_profile_request_json": "{}",
            "root": "C:/missing-store",
        })
    }

    fn content_seal(value: u8, byte_len: u64) -> AuthoringContentSeal {
        AuthoringContentSeal {
            byte_len,
            sha256: AuthoringSha256Digest::from_bytes([value; 32]),
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
        let project_id = ProjectId::from_bytes([0x70; 16]);
        let npc_id = EntityId::from_bytes([NPC_BYTE; 16]);
        let module_id = EntityId::from_bytes([MODULE_BYTE; 16]);
        let owner = Revision3TypedRef::new(project_id, npc_id, Revision3EntityKind::NpcDraft);
        let draft = Revision3NpcDraft {
            generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
            generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
            input: Revision3NpcDraftInput {
                target: target(),
                module_namespace: "GoreMods.Npcs.ProfileGuard".to_owned(),
                unique_name: "GORE_PROFILE_GUARD".to_owned(),
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
                name: "Native NPC profile FFI".to_owned(),
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
                        display_name: "Profile Guard".to_owned(),
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
                        display_name: "Profile Guard source".to_owned(),
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

    fn profile_request(
        project: &ProjectRevision3,
        head: &WorkingHead,
    ) -> Revision3NpcProfileEditRequestV1 {
        Revision3NpcProfileEditRequestV1 {
            expected_head: head.clone(),
            expected_project_id: project.project_id,
            expected_revision: project.revision,
            expected_target: project.target.clone(),
            expected_story_catalog_seal: content_seal(5, 5_001),
            expected_npc_catalog_seal: content_seal(6, 6_001),
            npc_id: EntityId::from_bytes([NPC_BYTE; 16]),
            expected_npc_revision: 2,
            script_module_id: EntityId::from_bytes([MODULE_BYTE; 16]),
            expected_script_module_revision: 3,
            expected_parent_catalog_id: CURRENT_PARENT.to_owned(),
            display_name: "Renamed Profile Guard".to_owned(),
            parent_catalog_id: CURRENT_PARENT.to_owned(),
        }
    }

    fn published_store(project: &ProjectRevision3) -> (TempDir, String, WorkingHead, Vec<u8>) {
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let project_json = project.to_canonical_json().unwrap();
        let prepared = store.prepare_revision3_checkpoint(None, project).unwrap();
        fs::write(temp.path().join("gore-project.json"), &prepared.head_bytes).unwrap();
        (temp, project_json, prepared.head, prepared.head_bytes)
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
    fn exact_raw_wire_and_public_dispatch_reject_ambiguous_authority() {
        let valid = raw_request(valid_shape());
        let parsed: PrepareNpcProfileWirePayload = parse_exact_wire(&valid).unwrap();
        assert_eq!(parsed.current_project_json, "{}");
        let cases = [
            format!(
                "{{\"command\":\"{COMMAND}\",\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"game_root\":\"g\",\"npc_profile_request_json\":\"{{}}\",\"root\":\"r\"}}}}"
            ),
            format!(
                "{{\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"game_root\":\"g\",\"game_root\":\"forged\",\"npc_profile_request_json\":\"{{}}\",\"root\":\"r\"}}}}"
            ),
            raw_request(json!({
                "current_project_json": "{}", "game_root": "g",
                "npc_profile_request_json": "{}", "root": "r", "authority": true,
            })),
            raw_request(json!({
                "game_root": "g", "npc_profile_request_json": "{}", "root": "r",
            })),
            raw_request(json!({
                "current_project_json": {}, "game_root": "g",
                "npc_profile_request_json": "{}", "root": "r",
            })),
            format!(" {valid}"),
        ];
        for input in cases {
            assert_eq!(
                prepare_revision3_npc_profile_edit_v1_raw(&input)["error"]["code"],
                "AUTHORING_REVISION3_NPC_PROFILE_REQUEST_INVALID",
                "{input}"
            );
            let public: Value = serde_json::from_str(&crate::execute_json(&input)).unwrap();
            assert_eq!(
                public["error"]["code"],
                "AUTHORING_REVISION3_NPC_PROFILE_REQUEST_INVALID"
            );
        }
    }

    #[test]
    fn nested_transports_paths_and_signed_wire_are_bounded_before_store_access() {
        let mut payload = valid_shape();
        payload["root"] = Value::String("x".repeat(MAX_PATH_BYTES + 1));
        assert_eq!(
            prepare_revision3_npc_profile_edit_v1_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_NPC_PROFILE_REQUEST_INVALID"
        );
        let mut payload = valid_shape();
        payload["current_project_json"] = Value::String("x".repeat(MAX_PROJECT_JSON_BYTES + 1));
        assert_eq!(
            prepare_revision3_npc_profile_edit_v1_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_NPC_PROFILE_PROJECT_LIMIT"
        );
        let mut payload = valid_shape();
        payload["npc_profile_request_json"] =
            Value::String("x".repeat(MAX_REVISION3_NPC_PROFILE_EDIT_REQUEST_JSON_BYTES_V1 + 1));
        assert_eq!(
            prepare_revision3_npc_profile_edit_v1_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_NPC_PROFILE_REQUEST_LIMIT"
        );
        assert_eq!(
            prepare_revision3_npc_profile_edit_v1_raw(&" ".repeat(MAX_WIRE_BYTES + 1))["error"]
                ["code"],
            "AUTHORING_REVISION3_NPC_PROFILE_INPUT_LIMIT"
        );
        assert_eq!(
            validate_basis_revision(MAX_BASIS_REVISION + 1)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_NPC_PROFILE_REVISION_LIMIT"
        );
    }

    #[test]
    fn malformed_nested_request_is_rejected_before_opening_a_store() {
        let temp = TempDir::new().unwrap();
        let missing = temp.path().join("missing-store");
        for nested in [
            "{}".to_owned(),
            " {\"expected_head\":null}".to_owned(),
            "{\"npc_id\":1,\"npc_id\":2}".to_owned(),
        ] {
            let response = prepare_revision3_npc_profile_edit_v1_raw(&raw_request(json!({
                "current_project_json": "{}",
                "game_root": temp.path().join("missing-game"),
                "npc_profile_request_json": nested,
                "root": missing,
            })));
            assert_eq!(
                response["error"]["code"],
                "AUTHORING_REVISION3_NPC_PROFILE_REQUEST_INVALID"
            );
        }
        assert!(!missing.exists());
    }

    #[test]
    fn project_drift_and_missing_game_never_publish_or_write_store_objects() {
        let project = npc_project(7);
        let (temp, project_json, head, fixed_head) = published_store(&project);
        let request = profile_request(&project, &head)
            .to_canonical_json()
            .unwrap();
        let before = snapshot_regular_files(temp.path());

        let drift = prepare_revision3_npc_profile_edit_v1_raw(&raw_request(json!({
            "current_project_json": project_json.replacen("\"revision\":7", "\"revision\":6", 1),
            "game_root": temp.path().join("missing-game"),
            "npc_profile_request_json": request,
            "root": temp.path(),
        })));
        assert_eq!(
            drift["error"]["code"],
            "AUTHORING_REVISION3_NPC_PROFILE_PROJECT_CONFLICT"
        );
        assert_eq!(snapshot_regular_files(temp.path()), before);

        let unavailable = prepare_revision3_npc_profile_edit_v1_raw(&raw_request(json!({
            "current_project_json": project_json,
            "game_root": temp.path().join("missing-game"),
            "npc_profile_request_json": profile_request(&project, &head).to_canonical_json().unwrap(),
            "root": temp.path(),
        })));
        assert!(!unavailable["ok"].as_bool().unwrap_or(false));
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            fixed_head
        );
        assert_eq!(snapshot_regular_files(temp.path()), before);
        let encoded = format!("{drift}{unavailable}");
        assert!(!encoded.contains(temp.path().to_string_lossy().as_ref()));
        assert!(!encoded.contains("game_root"));
    }

    #[test]
    fn fixed_basis_guard_rejects_an_external_publisher() {
        let project = npc_project(7);
        let (temp, _json, head, _fixed_head) = published_store(&project);
        let store = WorkingProjectStore::open_existing(temp.path(), ffi_store_limits()).unwrap();
        require_fixed_basis(&store, &head, &project).unwrap();

        let mut rival = project.clone();
        rival.meta.name = "External publisher won".to_owned();
        let rival = store
            .prepare_revision3_checkpoint(Some(&head), &rival)
            .unwrap();
        fs::write(temp.path().join("gore-project.json"), &rival.head_bytes).unwrap();
        assert_eq!(
            require_fixed_basis(&store, &head, &project)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_NPC_PROFILE_HEAD_CONFLICT"
        );
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            rival.head_bytes
        );
    }

    #[test]
    fn response_contract_is_exact_structural_unpublished_and_authority_free() {
        let response = json!({
            "ok": true,
            "outcome": "prepared_unpublished",
            "basis_head_json": "{}",
            "head_json": "{}",
            "project_json": "{}",
            "project_id": ProjectId::from_bytes([1; 16]).to_string(),
            "revision": 2,
            "npc_id": EntityId::from_bytes([2; 16]).to_string(),
            "npc_revision": 3,
            "script_module_id": EntityId::from_bytes([3; 16]).to_string(),
            "script_module_revision": 4,
            "display_name": "Renamed",
            "previous_parent_catalog_id": CURRENT_PARENT,
            "parent_catalog_id": DESIRED_PARENT,
            "story_catalog_seal": content_seal(5, 5),
            "npc_catalog_seal": content_seal(6, 6),
            "name_changed": true,
            "archetype_changed": true,
            "module_regenerated": true,
            "build_status": "blocked",
            "runtime_status": "runtime_unqualified",
            "catalog_authority": "not_granted",
            "collision_authority": "not_granted",
            "publication_status": "not_supported",
        });
        enforce_response_budget(&response).unwrap();
        let actual = response
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let expected = BTreeSet::from([
            "archetype_changed",
            "basis_head_json",
            "build_status",
            "catalog_authority",
            "collision_authority",
            "display_name",
            "head_json",
            "module_regenerated",
            "name_changed",
            "npc_catalog_seal",
            "npc_id",
            "npc_revision",
            "ok",
            "outcome",
            "parent_catalog_id",
            "previous_parent_catalog_id",
            "project_id",
            "project_json",
            "publication_status",
            "revision",
            "runtime_status",
            "script_module_id",
            "script_module_revision",
            "story_catalog_seal",
        ]);
        assert_eq!(actual, expected);
        let encoded = response.to_string();
        for forbidden in [
            "game_root",
            "root_path",
            "catalog_json",
            "collision_inventory",
            "source",
            "relative_path",
            "compile_ready",
            "\"publication_status\":\"published\"",
        ] {
            assert!(!encoded.contains(forbidden), "{forbidden}: {encoded}");
        }
    }

    #[test]
    fn transaction_conflicts_have_stable_retry_boundaries() {
        assert_eq!(
            map_transaction_conflict(Revision3NpcProfileEditConflictV1::CurrentHeadMismatch).code,
            "AUTHORING_REVISION3_NPC_PROFILE_HEAD_CONFLICT"
        );
        assert_eq!(
            map_transaction_conflict(Revision3NpcProfileEditConflictV1::NpcRevisionConflict {
                expected: 1,
                actual: 2,
            },)
            .code,
            "AUTHORING_REVISION3_NPC_PROFILE_NPC_CONFLICT"
        );
        assert_eq!(
            map_transaction_conflict(
                Revision3NpcProfileEditConflictV1::ScriptModuleRevisionConflict {
                    expected: 1,
                    actual: 2,
                },
            )
            .code,
            "AUTHORING_REVISION3_NPC_PROFILE_MODULE_CONFLICT"
        );
        assert_eq!(
            map_transaction_conflict(Revision3NpcProfileEditConflictV1::StoryCatalogSealMismatch,)
                .code,
            "AUTHORING_REVISION3_NPC_PROFILE_CATALOG_CONFLICT"
        );
        assert_eq!(
            map_transaction_conflict(Revision3NpcProfileEditConflictV1::NoChanges).code,
            "AUTHORING_REVISION3_NPC_PROFILE_NO_CHANGES"
        );
    }

    #[test]
    fn store_and_game_roots_must_be_disjoint() {
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
            "AUTHORING_REVISION3_NPC_PROFILE_STORE_GAME_ALIAS"
        );
        let direct_g1r = game_root.join("g1r");
        let sibling_store_root = game_root.join("projects");
        fs::create_dir(&direct_g1r).unwrap();
        fs::create_dir(&sibling_store_root).unwrap();
        let sibling =
            WorkingProjectStore::open_existing(&sibling_store_root, ffi_store_limits()).unwrap();
        assert_eq!(
            ensure_store_is_outside_game(&sibling, &direct_g1r)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_NPC_PROFILE_STORE_GAME_ALIAS"
        );
    }

    fn project_for_live_game(game_root: &Path) -> ProjectRevision3 {
        let g1r = resolve_g1r_root(game_root);
        let executable = g1r
            .join("Binaries")
            .join("Win64")
            .join("G1R-Win64-Shipping.exe");
        let binds_path = g1r.join("Script").join("Binds.Cache");
        let shipping = gore_mod::pristine_script_cache(game_root).unwrap();
        let catalog = build_known_catalog_with_shipping_snapshot(
            &executable,
            &shipping,
            &binds_path,
            GenerationInputLimits::default(),
        )
        .unwrap();
        catalog.revalidate_generation_inputs().unwrap();
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([0x91; 16]),
            revision: 0,
            meta: ProjectMeta {
                name: "Live NPC profile FFI".to_owned(),
                version: "1.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: authoring_target(catalog.generation()),
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn create_request(project: &ProjectRevision3, head: &WorkingHead) -> String {
        Revision3NpcDraftInsertRequestV1 {
            expected_head: head.clone(),
            expected_project_id: project.project_id,
            expected_revision: project.revision,
            expected_target: project.target.clone(),
            npc_id: EntityId::from_bytes([NPC_BYTE; 16]),
            script_module_id: EntityId::from_bytes([MODULE_BYTE; 16]),
            display_name: "Live Profile Guard".to_owned(),
            intent: Revision3NpcDraftIntentV1 {
                module_namespace: "GoreMods.Npcs.LiveProfileGuard".to_owned(),
                unique_name: "GORE_LIVE_PROFILE_GUARD".to_owned(),
                parent_catalog_id: CURRENT_PARENT.to_owned(),
            },
        }
        .to_canonical_json()
        .unwrap()
    }

    fn live_profile_request(
        project: &ProjectRevision3,
        head: &WorkingHead,
        story_seal: AuthoringContentSeal,
        npc_seal: AuthoringContentSeal,
        display_name: &str,
        current_parent: &str,
        desired_parent: &str,
    ) -> Revision3NpcProfileEditRequestV1 {
        let npc = &project.entities[&EntityId::from_bytes([NPC_BYTE; 16])];
        let Revision3EntityPayload::NpcDraft(draft) = &npc.payload else {
            panic!("expected NPC")
        };
        let module = &project.entities[&draft.script_module.id];
        Revision3NpcProfileEditRequestV1 {
            expected_head: head.clone(),
            expected_project_id: project.project_id,
            expected_revision: project.revision,
            expected_target: project.target.clone(),
            expected_story_catalog_seal: story_seal,
            expected_npc_catalog_seal: npc_seal,
            npc_id: npc.id,
            expected_npc_revision: npc.revision,
            script_module_id: module.id,
            expected_script_module_revision: module.revision,
            expected_parent_catalog_id: current_parent.to_owned(),
            display_name: display_name.to_owned(),
            parent_catalog_id: desired_parent.to_owned(),
        }
    }

    #[test]
    #[ignore = "requires GORE_STORY_GAME_ROOT pointing at a pinned supported game generation"]
    fn live_name_then_archetype_edit_reopens_without_publishing_or_granting_authority() {
        let game_root = PathBuf::from(
            std::env::var("GORE_STORY_GAME_ROOT")
                .expect("set GORE_STORY_GAME_ROOT for the live NPC profile FFI test"),
        );
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let base = project_for_live_game(&game_root);
        let base_json = base.to_canonical_json().unwrap();
        let base_head = store.prepare_revision3_checkpoint(None, &base).unwrap();
        fs::write(temp.path().join("gore-project.json"), &base_head.head_bytes).unwrap();

        let created = crate::authoring_story_npc_revision3::prepare_revision3_npc_draft_v1_raw(
            &json!({
                "command": "authoring_store_prepare_revision3_npc_draft_v1",
                "payload": {
                    "current_project_json": base_json,
                    "game_root": game_root,
                    "npc_request_json": create_request(&base, &base_head.head),
                    "root": temp.path(),
                },
            })
            .to_string(),
        );
        assert_eq!(created["ok"], true, "{created}");
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            base_head.head_bytes
        );

        // The route under test never publishes. This explicit write simulates its managed caller.
        let created_head_bytes = created["head_json"].as_str().unwrap().as_bytes().to_vec();
        fs::write(temp.path().join("gore-project.json"), &created_head_bytes).unwrap();
        let created_basis = store
            .open_current_revision3(AssetVerification::Full)
            .unwrap();
        let fresh = build_fresh_game_inputs(&game_root).unwrap();
        let story_seal = authoring_seal(fresh.story_catalog.catalog_seal());
        let npc_seal = authoring_seal(fresh.npc_catalog.catalog_seal());
        let rename = live_profile_request(
            &created_basis.project,
            &created_basis.head,
            story_seal.clone(),
            npc_seal.clone(),
            "Renamed Live Profile Guard",
            CURRENT_PARENT,
            CURRENT_PARENT,
        );
        let before_module =
            created_basis.project.entities[&EntityId::from_bytes([MODULE_BYTE; 16])].clone();
        let renamed = prepare_revision3_npc_profile_edit_v1_raw(&raw_request(json!({
            "current_project_json": created_basis.project.to_canonical_json().unwrap(),
            "game_root": game_root,
            "npc_profile_request_json": rename.to_canonical_json().unwrap(),
            "root": temp.path(),
        })));
        assert_eq!(renamed["ok"], true, "{renamed}");
        assert_eq!(renamed["outcome"], "prepared_unpublished");
        assert_eq!(renamed["name_changed"], true);
        assert_eq!(renamed["archetype_changed"], false);
        assert_eq!(renamed["module_regenerated"], false);
        assert_eq!(renamed["build_status"], "blocked");
        assert_eq!(renamed["runtime_status"], "runtime_unqualified");
        assert_eq!(renamed["catalog_authority"], "not_granted");
        assert_eq!(renamed["collision_authority"], "not_granted");
        assert_eq!(renamed["publication_status"], "not_supported");
        assert_eq!(renamed["story_catalog_seal"], json!(story_seal));
        assert_eq!(renamed["npc_catalog_seal"], json!(npc_seal));
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            created_head_bytes
        );
        let renamed_candidate = store
            .open_revision3_head_bytes(
                renamed["head_json"].as_str().unwrap().as_bytes(),
                AssetVerification::Full,
            )
            .unwrap();
        assert_eq!(
            renamed_candidate.project.entities[&EntityId::from_bytes([MODULE_BYTE; 16])],
            before_module
        );

        let renamed_head_bytes = renamed["head_json"].as_str().unwrap().as_bytes().to_vec();
        fs::write(temp.path().join("gore-project.json"), &renamed_head_bytes).unwrap();
        let archetype_basis = store
            .open_current_revision3(AssetVerification::Full)
            .unwrap();
        let archetype = live_profile_request(
            &archetype_basis.project,
            &archetype_basis.head,
            story_seal,
            npc_seal,
            "Renamed Live Profile Guard",
            CURRENT_PARENT,
            DESIRED_PARENT,
        );
        let before_module =
            archetype_basis.project.entities[&EntityId::from_bytes([MODULE_BYTE; 16])].clone();
        let edited = prepare_revision3_npc_profile_edit_v1_raw(&raw_request(json!({
            "current_project_json": archetype_basis.project.to_canonical_json().unwrap(),
            "game_root": game_root,
            "npc_profile_request_json": archetype.to_canonical_json().unwrap(),
            "root": temp.path(),
        })));
        assert_eq!(edited["ok"], true, "{edited}");
        assert_eq!(edited["name_changed"], false);
        assert_eq!(edited["archetype_changed"], true);
        assert_eq!(edited["module_regenerated"], true);
        assert_eq!(edited["previous_parent_catalog_id"], CURRENT_PARENT);
        assert_eq!(edited["parent_catalog_id"], DESIRED_PARENT);
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            renamed_head_bytes
        );
        let reopened = store
            .open_revision3_head_bytes(
                edited["head_json"].as_str().unwrap().as_bytes(),
                AssetVerification::Full,
            )
            .unwrap();
        let after_module = &reopened.project.entities[&EntityId::from_bytes([MODULE_BYTE; 16])];
        assert_eq!(after_module.revision, before_module.revision + 1);
        let Revision3EntityPayload::ScriptModule(before) = &before_module.payload else {
            panic!("expected prior module")
        };
        let Revision3EntityPayload::ScriptModule(after) = &after_module.payload else {
            panic!("expected edited module")
        };
        assert_eq!(after.generator_id, before.generator_id);
        assert_eq!(after.generator_version, before.generator_version);
        assert_eq!(after.owner, before.owner);
        assert_eq!(after.module_namespace, before.module_namespace);
        assert_eq!(after.module_relative_path, before.module_relative_path);
        assert_eq!(after.status, before.status);
        assert_ne!(after.source, before.source);

        let encoded = format!("{renamed}{edited}");
        assert!(!encoded.contains(temp.path().to_string_lossy().as_ref()));
        assert!(!encoded.contains(game_root.to_string_lossy().as_ref()));
        assert!(!encoded.contains("catalog_json"));
        assert!(!encoded.contains("collision_inventory"));
    }
}
