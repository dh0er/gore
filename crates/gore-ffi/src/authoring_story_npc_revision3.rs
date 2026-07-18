//! Native, prepare-only orchestration for one revision-3 NPC Draft transaction.
//!
//! The route rebuilds the pinned Story catalog, the broad sealed NPC archetype catalog, and a
//! base-game plus exact-current-project script collision inventory from fresh native/store inputs.
//! A curated offline-qualified parent selection must match one archetype linkage record exactly.
//! The resulting contexts are consumed by the filesystem-free authoring transaction and only a
//! fully reopened immutable checkpoint is returned. The fixed project head is never published.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use gore_authoring::{
    apply_revision3_npc_draft_transaction_v1, AssetVerification,
    ContentSeal as AuthoringContentSeal, GameGenerationAnchor,
    PreparedRevision3QuestCollisionSourceV2, Revision2NpcParentClassInput,
    Revision3NpcCatalogAuthorityV1, Revision3NpcCatalogSelectionV1,
    Revision3NpcCollisionAuthorityV1, Revision3NpcCollisionInventoryV1,
    Revision3NpcDraftBuildStatusV1, Revision3NpcDraftInsertConflictV1,
    Revision3NpcDraftInsertErrorV1, Revision3NpcDraftInsertEvaluationV1,
    Revision3NpcDraftInsertRequestV1, Revision3NpcDraftPublicationStatusV1,
    Revision3NpcDraftRuntimeStatusV1, Revision3NpcSourceInspectionStatusV1,
    Revision3QuestCollisionSourceErrorV2, Sha256Digest as AuthoringSha256Digest, WorkingHead,
    WorkingProjectStore, WorkingStoreError, WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
    MAX_REVISION3_NPC_DRAFT_REQUEST_JSON_BYTES_V1, REVISION3_NPC_EXACT_COLLISION_LAYER_V1,
};
use gore_npc_catalog::{build_npc_archetype_catalog, NpcArchetypeCatalogFile, NpcCatalogError};
use gore_story_catalog::{
    build_known_catalog_with_shipping_snapshot, AuthoringClassSelection, AuthoringNpcSelection,
    CatalogError, ContentSeal, GameGenerationSeal, GenerationInputLimits, StoryCatalogFile,
};
use gore_story_inventory::{
    build_base_game_inventory, QuestCollisionBuildStatus, QuestCollisionPublicationStatus,
    QuestCollisionRuntimeQualification, Revision3QuestCollisionCapabilityErrorV2,
    Revision3QuestCollisionCoverageV2, StoryInventoryError,
    VerifiedRevision3QuestCollisionCapabilityV2, MAX_BINDS_CACHE_SOURCE_BYTES,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

use crate::authoring_story_inventory::{read_source_no_follow, SourceReadError};
use crate::err;

pub(super) const COMMAND: &str = "authoring_store_prepare_revision3_npc_draft_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_BASIS_REVISION: u64 = i64::MAX as u64 - 1;
const MAX_WIRE_BYTES: usize = MAX_PROJECT_JSON_BYTES * 2
    + MAX_REVISION3_NPC_DRAFT_REQUEST_JSON_BYTES_V1 * 2
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
struct PrepareNpcWirePayload {
    current_project_json: String,
    game_root: String,
    npc_request_json: String,
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

struct FreshNpcInputs {
    story_catalog: StoryCatalogFile,
    npc_catalog: NpcArchetypeCatalogFile,
    collision_capability: VerifiedRevision3QuestCollisionCapabilityV2,
    shipping: Vec<u8>,
}

pub(super) fn prepare_revision3_npc_draft_v1_raw(input: &str) -> Value {
    prepare_revision3_npc_draft_v1_inner(input).unwrap_or_else(Failure::response)
}

fn prepare_revision3_npc_draft_v1_inner(input: &str) -> Result<Value, Failure> {
    let payload: PrepareNpcWirePayload = parse_exact_wire(input)?;
    validate_payload(&payload)?;

    let root = Path::new(&payload.root);
    let store =
        WorkingProjectStore::open_existing(root, ffi_store_limits()).map_err(map_store_error)?;
    let basis = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    validate_basis_revision(basis.project.revision)?;
    require_signed_serializable(&basis.project)?;
    let canonical_basis = basis.project.to_canonical_json().map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_STORE_INVARIANT",
            "the exact current revision-3 project could not be serialized",
        )
    })?;
    if canonical_basis.as_bytes() != payload.current_project_json.as_bytes() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROJECT_CONFLICT",
            "current_project_json differs from the exact published revision-3 project",
        ));
    }

    let request = Revision3NpcDraftInsertRequestV1::from_json(&payload.npc_request_json)
        .map_err(map_request_error)?;
    require_signed_serializable(&request)?;
    if request.expected_head != basis.head {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_HEAD_CONFLICT",
            "the NPC request head differs from the exact published revision-3 head",
        ));
    }

    let current_collision_source = store
        .prepare_current_revision3_quest_collision_source_v2(&basis.head)
        .map_err(map_current_collision_source_error)?;

    let game_root = PathBuf::from(&payload.game_root);
    ensure_store_is_outside_game(&store, &game_root)?;
    let fresh = build_fresh_game_inputs(&game_root, current_collision_source)?;
    let FreshNpcInputs {
        story_catalog,
        npc_catalog,
        collision_capability,
        shipping,
    } = fresh;
    let selection = resolve_native_selection(
        &story_catalog,
        &npc_catalog,
        &request.intent.parent_catalog_id,
    )?;
    let collision_inventory = native_collision_inventory(collision_capability, &story_catalog)?;

    let outcome = match apply_revision3_npc_draft_transaction_v1(
        &basis.head,
        &payload.current_project_json,
        &payload.npc_request_json,
        selection,
        collision_inventory,
    )
    .map_err(map_transaction_error)?
    {
        Revision3NpcDraftInsertEvaluationV1::Applied(outcome) => *outcome,
        Revision3NpcDraftInsertEvaluationV1::Rejected(rejection) => {
            return Err(map_transaction_conflict(rejection.conflict));
        }
    };
    require_signed_serializable(&outcome.project)?;
    if outcome.basis_head != basis.head
        || outcome.npc_id != request.npc_id
        || outcome.script_module_id != request.script_module_id
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_INVARIANT",
            "the native NPC transaction result changed its exact request binding",
        ));
    }

    // Close the mutable native-input window immediately before immutable Store installation.
    revalidate_game_inputs(&story_catalog, &game_root, &shipping)?;
    ensure_store_is_outside_game(&store, &game_root)?;
    let prepared = store
        .prepare_revision3_checkpoint(Some(&basis.head), &outcome.project)
        .map_err(map_store_error)?;
    let reopened = store
        .open_revision3_head_bytes(&prepared.head_bytes, AssetVerification::Full)
        .map_err(map_store_error)?;
    if reopened.head != prepared.head || reopened.project != outcome.project {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_STORE_INVARIANT",
            "the prepared revision-3 NPC checkpoint did not reopen exactly",
        ));
    }
    let current_after = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if current_after.head != basis.head || current_after.project != basis.project {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_HEAD_CONFLICT",
            "the published revision-3 project changed during NPC preparation",
        ));
    }
    // A late source race can leave only immutable CAS orphans. It never returns a candidate and
    // never changes the fixed project head.
    revalidate_game_inputs(&story_catalog, &game_root, &shipping)?;

    let basis_head_json = canonical_head_json(&basis.head)?;
    let candidate_head_json = String::from_utf8(prepared.head_bytes).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_STORE_INVARIANT",
            "the prepared revision-3 NPC head is not UTF-8 JSON",
        )
    })?;
    if candidate_head_json.is_empty() || candidate_head_json.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_RESPONSE_LIMIT",
            "the prepared revision-3 NPC head exceeds its transport limit",
        ));
    }
    require_signed_serializable(&prepared.head)?;
    let build_status = match outcome.build_status {
        Revision3NpcDraftBuildStatusV1::Blocked => "blocked",
    };
    let runtime_status = match outcome.runtime_status {
        Revision3NpcDraftRuntimeStatusV1::RuntimeUnqualified => "runtime_unqualified",
    };
    let catalog_authority = match outcome.catalog_authority {
        Revision3NpcCatalogAuthorityV1::NotGranted => "not_granted",
    };
    let collision_authority = match outcome.collision_authority {
        Revision3NpcCollisionAuthorityV1::NotGranted => "not_granted",
    };
    let source_inspection = match outcome.source_inspection {
        Revision3NpcSourceInspectionStatusV1::FreshNativeContextRequired => {
            "fresh_native_context_required"
        }
    };
    let publication_status = match outcome.publication_status {
        Revision3NpcDraftPublicationStatusV1::NotSupported => "not_supported",
    };
    let response = json!({
        "ok": true,
        "outcome": "prepared_unpublished",
        "basis_head_json": basis_head_json,
        "head_json": candidate_head_json,
        "project_json": outcome.canonical_project_json,
        "revision": outcome.project.revision,
        "npc_id": outcome.npc_id.to_string(),
        "script_module_id": outcome.script_module_id.to_string(),
        "build_status": build_status,
        "runtime_status": runtime_status,
        "catalog_authority": catalog_authority,
        "collision_authority": collision_authority,
        "source_inspection": source_inspection,
        "publication_status": publication_status,
    });
    enforce_response_budget(&response)?;
    let final_current = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if final_current.head != basis.head || final_current.project != basis.project {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_HEAD_CONFLICT",
            "the published revision-3 project changed before NPC preparation completed",
        ));
    }
    revalidate_game_inputs(&story_catalog, &game_root, &shipping)?;
    Ok(response)
}

fn parse_exact_wire<P: DeserializeOwned>(input: &str) -> Result<P, Failure> {
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_INPUT_LIMIT",
            format!("revision-3 NPC request exceeds the {MAX_WIRE_BYTES}-byte wire limit"),
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn validate_payload(payload: &PrepareNpcWirePayload) -> Result<(), Failure> {
    validate_path(&payload.root)?;
    validate_path(&payload.game_root)?;
    if payload.current_project_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.current_project_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_PROJECT_LIMIT",
            format!("current_project_json exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ));
    }
    if payload.npc_request_json.is_empty() {
        return Err(invalid_request());
    }
    if payload.npc_request_json.len() > MAX_REVISION3_NPC_DRAFT_REQUEST_JSON_BYTES_V1 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_REQUEST_LIMIT",
            format!(
                "npc_request_json exceeds the {MAX_REVISION3_NPC_DRAFT_REQUEST_JSON_BYTES_V1}-byte limit"
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
    current_collision_source: PreparedRevision3QuestCollisionSourceV2,
) -> Result<FreshNpcInputs, Failure> {
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
    let collision_inventory = build_base_game_inventory(&story_catalog, &shipping, &binds)
        .map_err(map_inventory_error)?;
    let collision_capability = VerifiedRevision3QuestCollisionCapabilityV2::bind(
        collision_inventory,
        &story_catalog,
        current_collision_source,
    )
    .map_err(map_collision_capability_error)?;
    let npc_catalog = build_npc_archetype_catalog(&story_catalog, &shipping, &binds)
        .map_err(map_npc_catalog_error)?;
    story_catalog
        .revalidate_generation_inputs()
        .map_err(map_catalog_error)?;
    Ok(FreshNpcInputs {
        story_catalog,
        npc_catalog,
        collision_capability,
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
            "AUTHORING_REVISION3_NPC_CATALOG_INVARIANT",
            "the native NPC archetype catalog is not bound to the exact Story catalog",
        ));
    }
    let selections = story_catalog
        .authoring_selections()
        .map_err(map_catalog_error)?;
    let selected = selections
        .npcs
        .iter()
        .find(|candidate| candidate.catalog_id == catalog_id)
        .ok_or_else(|| {
            Failure::new(
                "AUTHORING_REVISION3_NPC_CATALOG_SELECTION_INVALID",
                "the requested NPC parent catalog ID is unavailable",
            )
        })?;
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
            "AUTHORING_REVISION3_NPC_CATALOG_SELECTION_INVALID",
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
            "AUTHORING_REVISION3_NPC_CATALOG_SELECTION_UNQUALIFIED",
            "the requested NPC parent is not qualified for offline clone authoring",
        ));
    }
    Ok(())
}

fn native_collision_inventory(
    capability: VerifiedRevision3QuestCollisionCapabilityV2,
    story_catalog: &StoryCatalogFile,
) -> Result<Revision3NpcCollisionInventoryV1, Failure> {
    if capability.catalog_layer() != REVISION3_NPC_EXACT_COLLISION_LAYER_V1
        || capability.coverage()
            != Revision3QuestCollisionCoverageV2::BaseGameAndExactRevision3ProjectOnly
        || capability.runtime_qualification()
            != QuestCollisionRuntimeQualification::RuntimeUnqualified
        || capability.build_status() != QuestCollisionBuildStatus::Blocked
        || capability.publication_status() != QuestCollisionPublicationStatus::NotSupported
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_COLLISION_INVARIANT",
            "the native collision capability is not the closed exact-current offline layer",
        ));
    }
    if capability.story_catalog_seal() != story_catalog.catalog_seal() {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_COLLISION_INVARIANT",
            "the native collision capability is not bound to the selected Story catalog",
        ));
    }
    let authoring = story_catalog
        .authoring_selections()
        .map_err(map_catalog_error)?;
    if authoring.catalog_seal != *story_catalog.catalog_seal()
        || authoring.generation != *story_catalog.generation()
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_COLLISION_INVARIANT",
            "the native Story runtime-ID projection changed its catalog binding",
        ));
    }
    let mut catalog_runtime_ids = BTreeSet::new();
    for npc in authoring.npcs {
        let value = npc.runtime_unique_name;
        if value.is_empty()
            || value.len() > gore_authoring::MAX_REVISION3_COLLISION_IDENTITY_VALUE_BYTES_V2
            || !value.is_ascii()
            || value.bytes().any(|byte| byte.is_ascii_control())
            || !catalog_runtime_ids.insert(value.to_ascii_lowercase())
        {
            return Err(Failure::new(
                "AUTHORING_REVISION3_NPC_COLLISION_INVARIANT",
                "the native Story catalog has an invalid or duplicate runtime identity",
            ));
        }
    }
    let basis_head = capability.current_head().clone();
    let project_id = capability.project_id();
    let project_revision = capability.project_revision();
    let current_project = capability.current_project().clone();
    let generation = capability.project_target().clone();
    let story_catalog_seal = authoring_seal(capability.story_catalog_seal());
    let source_seal = authoring_seal(capability.combined_source_seal());
    let input = capability.into_quest_collision_input();
    if input.generation != generation
        || input.catalog_layer != REVISION3_NPC_EXACT_COLLISION_LAYER_V1
        || input.source_seal != source_seal
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_COLLISION_INVARIANT",
            "the consumed native collision capability changed its retained provenance",
        ));
    }
    Ok(Revision3NpcCollisionInventoryV1 {
        basis_head,
        project_id,
        project_revision,
        current_project,
        generation,
        story_catalog_seal,
        source_seal,
        catalog_layer: input.catalog_layer,
        catalog_runtime_ids,
        modules: input.modules,
        relative_paths: input.relative_paths,
        symbols: input.symbols,
    })
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
            "AUTHORING_REVISION3_NPC_STORE_PATH_UNSAFE",
            "the revision-3 working-store root could not be resolved safely",
        )
    })?;
    let install_root = fs::canonicalize(semantic_install_root(game_root)).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_INPUT_UNAVAILABLE",
            "the selected game installation root could not be resolved safely",
        )
    })?;
    if store_root.starts_with(&install_root) || install_root.starts_with(&store_root) {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_STORE_GAME_ALIAS",
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

fn validate_basis_revision(revision: u64) -> Result<(), Failure> {
    if revision > MAX_BASIS_REVISION {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_REVISION_LIMIT",
            format!("the published basis revision exceeds {MAX_BASIS_REVISION}"),
        ));
    }
    Ok(())
}

fn require_signed_serializable(value: &impl Serialize) -> Result<(), Failure> {
    let value = serde_json::to_value(value).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_INVARIANT",
            "a revision-3 NPC wire value could not be inspected",
        )
    })?;
    require_signed_json_value(&value)
}

fn require_signed_json_value(value: &Value) -> Result<(), Failure> {
    match value {
        Value::Number(number) if number.as_u64().is_some_and(|value| value > i64::MAX as u64) => {
            Err(Failure::new(
                "AUTHORING_REVISION3_NPC_SIGNED_WIRE_LIMIT",
                "a revision-3 NPC wire integer exceeds the signed 64-bit transport range",
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
            "AUTHORING_REVISION3_NPC_INVARIANT",
            "the revision-3 NPC basis head could not be serialized",
        )
    })?;
    if value.is_empty() || value.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_RESPONSE_LIMIT",
            "the revision-3 NPC basis head exceeds its transport limit",
        ));
    }
    Ok(value)
}

fn enforce_response_budget(response: &Value) -> Result<(), Failure> {
    let encoded = serde_json::to_vec(response).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_NPC_INVARIANT",
            "the revision-3 NPC response could not be serialized",
        )
    })?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_NPC_RESPONSE_LIMIT",
            "the revision-3 NPC response exceeds its bounded transport budget",
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
        "AUTHORING_REVISION3_NPC_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and exactly current_project_json, game_root, npc_request_json, and root",
    )
}

fn input_changed() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_NPC_INPUT_CHANGED",
        "the native game generation changed during revision-3 NPC preparation",
    )
}

fn map_request_error(error: impl std::fmt::Display) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_NPC_REQUEST_INVALID",
        format!("the exact revision-3 NPC request is invalid: {error}"),
    )
}

fn map_transaction_error(error: Revision3NpcDraftInsertErrorV1) -> Failure {
    match error {
        Revision3NpcDraftInsertErrorV1::InvalidProject(_) => Failure::new(
            "AUTHORING_REVISION3_NPC_PROJECT_INVALID",
            "the exact current revision-3 project is invalid",
        ),
        Revision3NpcDraftInsertErrorV1::InvalidRequest(error) => map_request_error(error),
        Revision3NpcDraftInsertErrorV1::ReopenCandidate(_)
        | Revision3NpcDraftInsertErrorV1::CanonicalReopenMismatch => Failure::new(
            "AUTHORING_REVISION3_NPC_INVARIANT",
            "the revision-3 NPC candidate failed exact canonical reopen",
        ),
    }
}

fn map_transaction_conflict(error: Revision3NpcDraftInsertConflictV1) -> Failure {
    let code = match &error {
        Revision3NpcDraftInsertConflictV1::CurrentHeadMismatch => {
            "AUTHORING_REVISION3_NPC_HEAD_CONFLICT"
        }
        Revision3NpcDraftInsertConflictV1::ProjectIdentityMismatch { .. }
        | Revision3NpcDraftInsertConflictV1::ProjectRevisionConflict { .. }
        | Revision3NpcDraftInsertConflictV1::ProjectTargetMismatch => {
            "AUTHORING_REVISION3_NPC_PROJECT_CONFLICT"
        }
        Revision3NpcDraftInsertConflictV1::StoryIdentityCollision { .. }
        | Revision3NpcDraftInsertConflictV1::EntityIdCollision { .. }
        | Revision3NpcDraftInsertConflictV1::SharedEntityId => "AUTHORING_REVISION3_NPC_COLLISION",
        Revision3NpcDraftInsertConflictV1::CatalogSelectionMismatch
        | Revision3NpcDraftInsertConflictV1::CatalogGenerationMismatch
        | Revision3NpcDraftInsertConflictV1::InvalidCatalogSelection
        | Revision3NpcDraftInsertConflictV1::InvalidCatalogId => {
            "AUTHORING_REVISION3_NPC_CATALOG_SELECTION_INVALID"
        }
        Revision3NpcDraftInsertConflictV1::CollisionGenerationMismatch
        | Revision3NpcDraftInsertConflictV1::CollisionBasisMismatch
        | Revision3NpcDraftInsertConflictV1::CollisionStoryCatalogMismatch
        | Revision3NpcDraftInsertConflictV1::CollisionLayerMismatch
        | Revision3NpcDraftInsertConflictV1::InvalidCollisionInventory => {
            "AUTHORING_REVISION3_NPC_COLLISION_INVARIANT"
        }
        Revision3NpcDraftInsertConflictV1::ProjectRevisionOverflow
        | Revision3NpcDraftInsertConflictV1::EntityCapacityExceeded => {
            "AUTHORING_REVISION3_NPC_LIMIT"
        }
        Revision3NpcDraftInsertConflictV1::ZeroEntityId { .. }
        | Revision3NpcDraftInsertConflictV1::InvalidDisplayName
        | Revision3NpcDraftInsertConflictV1::InvalidNpcIntent { .. } => {
            "AUTHORING_REVISION3_NPC_INTENT_INVALID"
        }
        Revision3NpcDraftInsertConflictV1::InvalidBasisStoryState { .. }
        | Revision3NpcDraftInsertConflictV1::CandidateNotPersistable { .. } => {
            "AUTHORING_REVISION3_NPC_PROJECT_INVALID"
        }
    };
    Failure::new(code, error.to_string())
}

fn map_pristine_error(error: gore_mod::ModError) -> Failure {
    let message = error.to_string();
    if message.contains("RECOVERY_REQUIRED") {
        return Failure::new(
            "AUTHORING_REVISION3_NPC_RECOVERY_REQUIRED",
            "an interrupted deployment must be recovered before NPC authoring",
        );
    }
    if message.contains("exceeds the") || message.contains("too large") {
        return Failure::new(
            "AUTHORING_REVISION3_NPC_INPUT_LIMIT",
            "the pristine Shipping cache exceeds its bounded input limit",
        );
    }
    if message.contains("not a regular non-link file") {
        return Failure::new(
            "AUTHORING_REVISION3_NPC_INPUT_UNSAFE",
            "the pristine Shipping cache is not a safe regular file",
        );
    }
    Failure::new(
        "AUTHORING_REVISION3_NPC_PRISTINE_UNAVAILABLE",
        "the pristine Shipping cache could not be selected safely",
    )
}

fn map_catalog_error(error: CatalogError) -> Failure {
    match error {
        CatalogError::InvalidLimits(_) | CatalogError::LimitExceeded { .. } => Failure::new(
            "AUTHORING_REVISION3_NPC_INPUT_LIMIT",
            "a native game-generation input exceeds its bounded resource limit",
        ),
        CatalogError::UnsafeInput(_) | CatalogError::OutputAliasesInput { .. } => Failure::new(
            "AUTHORING_REVISION3_NPC_INPUT_UNSAFE",
            "a native game-generation input is unsafe",
        ),
        CatalogError::IdentityChanged(_) | CatalogError::SourceChanged { .. } => input_changed(),
        CatalogError::UnsupportedGeneration { .. } => Failure::new(
            "AUTHORING_REVISION3_NPC_UNSUPPORTED_GENERATION",
            "the selected game does not match the supported pinned generation",
        ),
        CatalogError::Io { source, .. } if source.kind() == io::ErrorKind::NotFound => {
            Failure::new(
                "AUTHORING_REVISION3_NPC_INPUT_MISSING",
                "a required native game-generation input does not exist",
            )
        }
        _ => Failure::new(
            "AUTHORING_REVISION3_NPC_INPUT_UNAVAILABLE",
            "the native game generation could not be verified safely",
        ),
    }
}

fn map_source_read_error(error: SourceReadError) -> Failure {
    match error {
        SourceReadError::Missing => Failure::new(
            "AUTHORING_REVISION3_NPC_INPUT_MISSING",
            "a required native game-generation input does not exist",
        ),
        SourceReadError::Unsafe => Failure::new(
            "AUTHORING_REVISION3_NPC_INPUT_UNSAFE",
            "a native game-generation input is unsafe",
        ),
        SourceReadError::Limit => Failure::new(
            "AUTHORING_REVISION3_NPC_INPUT_LIMIT",
            "a native game-generation input exceeds its bounded resource limit",
        ),
        SourceReadError::Changed => input_changed(),
        SourceReadError::Io => Failure::new(
            "AUTHORING_REVISION3_NPC_INPUT_UNAVAILABLE",
            "a native game-generation input could not be read safely",
        ),
    }
}

fn map_inventory_error(error: StoryInventoryError) -> Failure {
    match error {
        StoryInventoryError::LimitExceeded { .. } | StoryInventoryError::SourcePairTooLarge => {
            Failure::new(
                "AUTHORING_REVISION3_NPC_COLLISION_LIMIT",
                "the trusted base-game collision inventory exceeds its resource limit",
            )
        }
        StoryInventoryError::UnsupportedGeneration => Failure::new(
            "AUTHORING_REVISION3_NPC_UNSUPPORTED_GENERATION",
            "the selected game does not match the supported pinned generation",
        ),
        StoryInventoryError::SourceLengthMismatch { .. }
        | StoryInventoryError::SourceDigestMismatch { .. }
        | StoryInventoryError::SourcePairSealMismatch
        | StoryInventoryError::RecollectedInventoryMismatch => input_changed(),
        _ => Failure::new(
            "AUTHORING_REVISION3_NPC_COLLISION_FAILED",
            "the trusted base-game collision inventory could not be rebuilt",
        ),
    }
}

fn map_current_collision_source_error(error: Revision3QuestCollisionSourceErrorV2) -> Failure {
    match error {
        Revision3QuestCollisionSourceErrorV2::Store(error) => map_store_error(error),
        Revision3QuestCollisionSourceErrorV2::CurrentSnapshotDrift => Failure::new(
            "AUTHORING_REVISION3_NPC_HEAD_CONFLICT",
            "the current revision-3 project changed while its collision source was prepared",
        ),
        Revision3QuestCollisionSourceErrorV2::Limit { .. }
        | Revision3QuestCollisionSourceErrorV2::TooManyPriorQuests { .. } => Failure::new(
            "AUTHORING_REVISION3_NPC_COLLISION_LIMIT",
            "the exact current-project collision source exceeds its bounded resource limit",
        ),
        Revision3QuestCollisionSourceErrorV2::InvalidCurrentProject { .. }
        | Revision3QuestCollisionSourceErrorV2::SharedQuestModule { .. }
        | Revision3QuestCollisionSourceErrorV2::ResidualQuestState { .. }
        | Revision3QuestCollisionSourceErrorV2::HistoricalQuestArtifactCrossReference { .. }
        | Revision3QuestCollisionSourceErrorV2::QuestOwnerDrift { .. }
        | Revision3QuestCollisionSourceErrorV2::ModuleOriginDrift { .. }
        | Revision3QuestCollisionSourceErrorV2::ForeignGenerator { .. }
        | Revision3QuestCollisionSourceErrorV2::SourceHashMismatch { .. }
        | Revision3QuestCollisionSourceErrorV2::InputFingerprintMismatch { .. }
        | Revision3QuestCollisionSourceErrorV2::PersistedModuleDrift { .. }
        | Revision3QuestCollisionSourceErrorV2::DuplicateRuntimeId { .. }
        | Revision3QuestCollisionSourceErrorV2::PriorIdentityCollision { .. }
        | Revision3QuestCollisionSourceErrorV2::NonQuestBasisInvalid { .. } => Failure::new(
            "AUTHORING_REVISION3_NPC_PROJECT_INVALID",
            "the exact current revision-3 project is not a valid NPC-authoring basis",
        ),
    }
}

fn map_collision_capability_error(error: Revision3QuestCollisionCapabilityErrorV2) -> Failure {
    match error {
        Revision3QuestCollisionCapabilityErrorV2::Catalog(error) => map_catalog_error(error),
        Revision3QuestCollisionCapabilityErrorV2::TargetMismatch => Failure::new(
            "AUTHORING_REVISION3_NPC_PROJECT_TARGET_MISMATCH",
            "the exact current project does not target the trusted game generation",
        ),
        Revision3QuestCollisionCapabilityErrorV2::Limit { .. } => Failure::new(
            "AUTHORING_REVISION3_NPC_COLLISION_LIMIT",
            "the combined base/current collision authority exceeds its bounded resource limit",
        ),
        Revision3QuestCollisionCapabilityErrorV2::BaseCurrentCollision { .. } => Failure::new(
            "AUTHORING_REVISION3_NPC_COLLISION",
            "an exact-current project Story identity collides with the trusted base game",
        ),
        Revision3QuestCollisionCapabilityErrorV2::PriorQuestParentDrift { .. }
        | Revision3QuestCollisionCapabilityErrorV2::PriorQuestGiverDrift { .. }
        | Revision3QuestCollisionCapabilityErrorV2::CurrentIdentityCollision { .. }
        | Revision3QuestCollisionCapabilityErrorV2::InvalidCollisionIdentity { .. } => {
            Failure::new(
                "AUTHORING_REVISION3_NPC_PROJECT_INVALID",
                "the exact current revision-3 project cannot form a closed collision basis",
            )
        }
        Revision3QuestCollisionCapabilityErrorV2::CatalogBindingMismatch
        | Revision3QuestCollisionCapabilityErrorV2::SourceBindingDrift { .. }
        | Revision3QuestCollisionCapabilityErrorV2::UnknownParent(_)
        | Revision3QuestCollisionCapabilityErrorV2::UnknownGiver(_)
        | Revision3QuestCollisionCapabilityErrorV2::InvalidCatalogQuery { .. }
        | Revision3QuestCollisionCapabilityErrorV2::Serialize(_) => Failure::new(
            "AUTHORING_REVISION3_NPC_COLLISION_INVARIANT",
            "fresh exact-current collision authority could not be bound exactly",
        ),
    }
}

fn map_npc_catalog_error(error: NpcCatalogError) -> Failure {
    match error {
        NpcCatalogError::LimitExceeded { .. } => Failure::new(
            "AUTHORING_REVISION3_NPC_CATALOG_LIMIT",
            "the native NPC archetype catalog exceeds its resource limit",
        ),
        NpcCatalogError::UnsupportedGeneration => Failure::new(
            "AUTHORING_REVISION3_NPC_UNSUPPORTED_GENERATION",
            "the selected game does not match the supported pinned generation",
        ),
        NpcCatalogError::GenerationInputMismatch { .. } => input_changed(),
        _ => Failure::new(
            "AUTHORING_REVISION3_NPC_CATALOG_FAILED",
            "the native NPC archetype catalog could not be rebuilt",
        ),
    }
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::InvalidLimits(_) => "AUTHORING_REVISION3_NPC_STORE_LIMITS_INVALID",
        WorkingStoreError::MissingRoot(_) => "AUTHORING_REVISION3_NPC_STORE_ROOT_MISSING",
        WorkingStoreError::UnsafePath { .. } => "AUTHORING_REVISION3_NPC_STORE_PATH_UNSAFE",
        WorkingStoreError::LimitExceeded { .. } => "AUTHORING_REVISION3_NPC_STORE_LIMIT",
        WorkingStoreError::HeadConflict { .. } => "AUTHORING_REVISION3_NPC_HEAD_CONFLICT",
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_NPC_HEAD_MISSING",
        WorkingStoreError::MissingObject(_) => "AUTHORING_REVISION3_NPC_STORE_OBJECT_MISSING",
        WorkingStoreError::SealMismatch { .. } => "AUTHORING_REVISION3_NPC_STORE_SEAL_MISMATCH",
        WorkingStoreError::Collision { .. } => "AUTHORING_REVISION3_NPC_STORE_COLLISION",
        WorkingStoreError::InvalidJson { .. } | WorkingStoreError::NonCanonicalJson { .. } => {
            "AUTHORING_REVISION3_NPC_STORE_JSON_INVALID"
        }
        WorkingStoreError::Invariant(_)
        | WorkingStoreError::InvalidOgg(_)
        | WorkingStoreError::OggMetadataMismatch { .. } => {
            "AUTHORING_REVISION3_NPC_STORE_INVARIANT"
        }
        WorkingStoreError::StagingCleanup { .. } | WorkingStoreError::Io(_) => {
            "AUTHORING_REVISION3_NPC_STORE_IO"
        }
    };
    Failure::new(code, "the revision-3 NPC working-store operation failed")
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
        AssetStoreIndex, FormatV2, ProjectId, ProjectMeta, ProjectRevision3, SchemaRevisionV3,
    };
    use tempfile::TempDir;

    use super::*;

    fn raw_request(payload: Value) -> String {
        json!({"command": COMMAND, "payload": payload}).to_string()
    }

    fn valid_shape() -> Value {
        json!({
            "current_project_json": "{}",
            "game_root": "C:/missing-game",
            "npc_request_json": "{}",
            "root": "C:/missing-store",
        })
    }

    fn project_at_revision(revision: u64) -> ProjectRevision3 {
        let executable = ContentSeal {
            byte_len: 123,
            sha256: gore_story_catalog::Sha256Digest::from_bytes([0x45; 32]),
        };
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([0x93; 16]),
            revision,
            meta: ProjectMeta {
                name: "Native NPC FFI".to_owned(),
                version: "1.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: GameGenerationAnchor {
                executable: authoring_seal(&executable),
            },
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex {
                assets: BTreeMap::new(),
            },
        }
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

        let mut project = project_at_revision(0);
        project.target = authoring_target(catalog.generation());
        project
    }

    fn published_store_at_revision(revision: u64) -> (TempDir, String, WorkingHead, Vec<u8>) {
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let project = project_at_revision(revision);
        let project_json = project.to_canonical_json().unwrap();
        let prepared = store.prepare_revision3_checkpoint(None, &project).unwrap();
        fs::write(temp.path().join("gore-project.json"), &prepared.head_bytes).unwrap();
        (temp, project_json, prepared.head, prepared.head_bytes)
    }

    fn request_json(project: &ProjectRevision3, basis_head: &WorkingHead, ordinal: u8) -> String {
        Revision3NpcDraftInsertRequestV1 {
            expected_head: basis_head.clone(),
            expected_project_id: project.project_id,
            expected_revision: project.revision,
            expected_target: project.target.clone(),
            npc_id: gore_authoring::EntityId::from_bytes([0x80 + ordinal * 2; 16]),
            script_module_id: gore_authoring::EntityId::from_bytes([0x81 + ordinal * 2; 16]),
            display_name: format!("Native NPC {ordinal}"),
            intent: gore_authoring::Revision3NpcDraftIntentV1 {
                module_namespace: format!("GoreMods.Npcs.NativeNpc{ordinal}"),
                unique_name: format!("GORE_NATIVE_NPC_{ordinal}"),
                parent_catalog_id: "g1r:npc:om_grd_asghan_263".to_owned(),
            },
        }
        .to_canonical_json()
        .unwrap()
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
        let parsed: PrepareNpcWirePayload = parse_exact_wire(&raw_request(valid_shape())).unwrap();
        assert_eq!(parsed.current_project_json, "{}");
        let cases = [
            format!(
                "{{\"command\":\"{COMMAND}\",\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"game_root\":\"g\",\"npc_request_json\":\"{{}}\",\"root\":\"r\"}}}}"
            ),
            format!(
                "{{\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"game_root\":\"g\",\"game_root\":\"forged\",\"npc_request_json\":\"{{}}\",\"root\":\"r\"}}}}"
            ),
            raw_request(json!({
                "current_project_json": "{}", "game_root": "g",
                "npc_request_json": "{}", "root": "r", "authority": "forged"
            })),
            raw_request(json!({
                "game_root": "g", "npc_request_json": "{}", "root": "r"
            })),
            raw_request(json!({
                "current_project_json": {}, "game_root": "g",
                "npc_request_json": "{}", "root": "r"
            })),
        ];
        for input in cases {
            assert_eq!(
                prepare_revision3_npc_draft_v1_raw(&input)["error"]["code"],
                "AUTHORING_REVISION3_NPC_REQUEST_INVALID"
            );
        }
    }

    #[test]
    fn public_dispatch_preserves_duplicate_rejection_for_raw_route() {
        let duplicate = format!(
            "{{\"command\":\"{COMMAND}\",\"payload\":{{\"current_project_json\":\"{{}}\",\"game_root\":\"g\",\"game_root\":\"forged\",\"npc_request_json\":\"{{}}\",\"root\":\"r\"}}}}"
        );
        let response: Value = serde_json::from_str(&crate::execute_json(&duplicate)).unwrap();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_NPC_REQUEST_INVALID"
        );
    }

    #[test]
    fn nested_transports_and_paths_are_bounded_before_filesystem_access() {
        let mut payload = valid_shape();
        payload["root"] = Value::String("x".repeat(MAX_PATH_BYTES + 1));
        assert_eq!(
            prepare_revision3_npc_draft_v1_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_NPC_REQUEST_INVALID"
        );
        let mut payload = valid_shape();
        payload["current_project_json"] = Value::String("x".repeat(MAX_PROJECT_JSON_BYTES + 1));
        assert_eq!(
            prepare_revision3_npc_draft_v1_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_NPC_PROJECT_LIMIT"
        );
        let mut payload = valid_shape();
        payload["npc_request_json"] =
            Value::String("x".repeat(MAX_REVISION3_NPC_DRAFT_REQUEST_JSON_BYTES_V1 + 1));
        assert_eq!(
            prepare_revision3_npc_draft_v1_raw(&raw_request(payload))["error"]["code"],
            "AUTHORING_REVISION3_NPC_REQUEST_LIMIT"
        );
    }

    #[test]
    fn mismatched_project_and_missing_game_never_change_published_head() {
        let (temp, project_json, basis_head, fixed_head) = published_store_at_revision(7);
        let project = ProjectRevision3::from_json(&project_json).unwrap();
        let before = snapshot_regular_files(temp.path());

        let response = prepare_revision3_npc_draft_v1_raw(&raw_request(json!({
            "current_project_json": project_json.replacen("\"revision\":7", "\"revision\":6", 1),
            "game_root": temp.path().join("missing-game"),
            "npc_request_json": request_json(&project, &basis_head, 1),
            "root": temp.path(),
        })));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_NPC_PROJECT_CONFLICT"
        );
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            fixed_head
        );
        assert_eq!(snapshot_regular_files(temp.path()), before);

        let response = prepare_revision3_npc_draft_v1_raw(&raw_request(json!({
            "current_project_json": project_json,
            "game_root": temp.path().join("missing-game"),
            "npc_request_json": request_json(&project, &basis_head, 1),
            "root": temp.path(),
        })));
        assert!(!response["ok"].as_bool().unwrap_or(false));
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            fixed_head
        );
        assert!(!response
            .to_string()
            .contains(temp.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn basis_revision_is_signed_safe_and_rejected_before_cas_writes() {
        assert!(validate_basis_revision(MAX_BASIS_REVISION).is_ok());
        assert_eq!(
            validate_basis_revision(MAX_BASIS_REVISION + 1)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_NPC_REVISION_LIMIT"
        );
        let (temp, project_json, basis_head, fixed_head) =
            published_store_at_revision(MAX_BASIS_REVISION + 1);
        let project = ProjectRevision3::from_json(&project_json).unwrap();
        let before = snapshot_regular_files(temp.path());
        let response = prepare_revision3_npc_draft_v1_raw(&raw_request(json!({
            "current_project_json": project_json,
            "game_root": temp.path().join("missing-game"),
            "npc_request_json": request_json(&project, &basis_head, 1),
            "root": temp.path(),
        })));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_NPC_REVISION_LIMIT"
        );
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            fixed_head
        );
        assert_eq!(snapshot_regular_files(temp.path()), before);
    }

    #[test]
    fn response_contract_is_structural_unpublished_and_path_free() {
        let response = json!({
            "ok": true,
            "outcome": "prepared_unpublished",
            "build_status": "blocked",
            "runtime_status": "runtime_unqualified",
            "catalog_authority": "not_granted",
            "collision_authority": "not_granted",
            "source_inspection": "fresh_native_context_required",
            "publication_status": "not_supported",
        });
        enforce_response_budget(&response).unwrap();
        assert_ne!(response["publication_status"], "published");
        assert!(response.get("catalog_json").is_none());
        assert!(response.get("collision_inventory_json").is_none());
        assert!(response.get("game_root").is_none());
        assert!(response.get("compile_ready").is_none());
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
            "AUTHORING_REVISION3_NPC_STORE_GAME_ALIAS"
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
            "AUTHORING_REVISION3_NPC_STORE_GAME_ALIAS"
        );
    }

    #[test]
    #[ignore = "requires GORE_STORY_GAME_ROOT pointing at the pinned supported game generation"]
    fn live_native_path_covers_native_and_prior_npc_collisions_without_publishing_candidate() {
        let game_root = std::env::var("GORE_STORY_GAME_ROOT")
            .expect("set GORE_STORY_GAME_ROOT for the live revision-3 NPC FFI test");
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let project = project_for_live_game(Path::new(&game_root));
        let project_json = project.to_canonical_json().unwrap();
        let published = store.prepare_revision3_checkpoint(None, &project).unwrap();
        fs::write(temp.path().join("gore-project.json"), &published.head_bytes).unwrap();

        let mut native_collision = Revision3NpcDraftInsertRequestV1::from_json(&request_json(
            &project,
            &published.head,
            1,
        ))
        .unwrap();
        native_collision.intent.module_namespace = "GoreMods.Npcs.NativeIdCollision".to_owned();
        native_collision.intent.unique_name = "OM_STT_Viper_302".to_owned();
        let native_collision_response = prepare_revision3_npc_draft_v1_raw(&raw_request(json!({
            "current_project_json": project_json,
            "game_root": game_root,
            "npc_request_json": native_collision.to_canonical_json().unwrap(),
            "root": temp.path(),
        })));
        assert_eq!(
            native_collision_response["error"]["code"], "AUTHORING_REVISION3_NPC_COLLISION",
            "{native_collision_response}"
        );
        assert!(native_collision_response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("AuthoredRuntimeId"));
        assert!(native_collision_response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("OM_STT_Viper_302"));
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            published.head_bytes
        );

        let first = prepare_revision3_npc_draft_v1_raw(&raw_request(json!({
            "current_project_json": project.to_canonical_json().unwrap(),
            "game_root": game_root,
            "npc_request_json": request_json(&project, &published.head, 1),
            "root": temp.path(),
        })));
        assert_eq!(first["ok"], true, "{first}");
        assert_eq!(first["outcome"], "prepared_unpublished");
        assert_eq!(first["revision"], 1);
        assert_eq!(first["build_status"], "blocked");
        assert_eq!(first["runtime_status"], "runtime_unqualified");
        assert_eq!(first["publication_status"], "not_supported");
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            published.head_bytes
        );
        let first_reopened = store
            .open_revision3_head_bytes(
                first["head_json"].as_str().unwrap().as_bytes(),
                AssetVerification::Full,
            )
            .unwrap();
        assert_eq!(first_reopened.project.entities.len(), 2);

        // Simulate a separate exact-CAS publisher. The NPC route itself never performs this write.
        fs::write(
            temp.path().join("gore-project.json"),
            first["head_json"].as_str().unwrap().as_bytes(),
        )
        .unwrap();
        let mut prior_collision = Revision3NpcDraftInsertRequestV1::from_json(&request_json(
            &first_reopened.project,
            &first_reopened.head,
            2,
        ))
        .unwrap();
        prior_collision.intent.module_namespace = "GoreMods.Npcs.NativeNpc1".to_owned();
        let prior_collision_response = prepare_revision3_npc_draft_v1_raw(&raw_request(json!({
            "current_project_json": first["project_json"],
            "game_root": game_root,
            "npc_request_json": prior_collision.to_canonical_json().unwrap(),
            "root": temp.path(),
        })));
        assert_eq!(
            prior_collision_response["error"]["code"], "AUTHORING_REVISION3_NPC_COLLISION",
            "{prior_collision_response}"
        );

        let second = prepare_revision3_npc_draft_v1_raw(&raw_request(json!({
            "current_project_json": first["project_json"],
            "game_root": game_root,
            "npc_request_json": request_json(&first_reopened.project, &first_reopened.head, 2),
            "root": temp.path(),
        })));
        assert_eq!(second["ok"], true, "{second}");
        assert_eq!(second["revision"], 2);
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            first["head_json"].as_str().unwrap().as_bytes()
        );
        let second_reopened = store
            .open_revision3_head_bytes(
                second["head_json"].as_str().unwrap().as_bytes(),
                AssetVerification::Full,
            )
            .unwrap();
        assert_eq!(second_reopened.project.entities.len(), 4);

        let encoded =
            format!("{native_collision_response}{first}{prior_collision_response}{second}");
        assert!(!encoded.contains(temp.path().to_string_lossy().as_ref()));
        assert!(!encoded.contains(&std::env::var("GORE_STORY_GAME_ROOT").unwrap()));
    }
}
