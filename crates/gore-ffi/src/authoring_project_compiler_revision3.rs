//! Exact-current, evidence-only compiler check for every managed revision-3 Story module.
//!
//! The caller selects only one working Store/head and one game installation. Native code closes
//! the complete Quest/NPC -> ScriptModule graph, regenerates every source from that exact snapshot,
//! seals a deterministic private manifest, and submits all modules to one shared compiler run.
//! No source, module path, compiler output, build plan, or deployment authority crosses the wire.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

use gore_as::compile::{
    acquire_compile_install_mutation, check_project_modules_with_diagnostics_report_with_guard,
    InstallMutationGuard, InstallRestoreDisposition, ProjectCompileOverlay,
    ProjectCompilerCheckOpts, ProjectCompilerCheckOutcome, ProjectCompilerCheckReport,
    ProjectCompilerClosingAuditDisposition, ProjectCompilerOutputDisposition,
    MAX_PROJECT_COMPILER_CHECK_MODULES,
};
use gore_as::diagnostics::DiagnosticsOptions;
use gore_authoring::{
    AssetVerification, ContentSeal, EntityId, ProjectRevision3, Revision3EntityKind,
    Revision3EntityPayload, Revision3OriginRef, Revision3ScriptModule, Revision3TypedRef,
    Sha256Digest, WorkingHead, WorkingProjectStore, WorkingStoreError, WorkingStoreLimits,
    LOGICAL_NPC_CLONE_GENERATOR_ID, LOGICAL_NPC_CLONE_GENERATOR_VERSION, MAX_PROJECT_JSON_BYTES,
    REVISION3_QUEST_GENERATOR_ID, REVISION3_QUEST_GENERATOR_VERSION,
};
use gore_story_build::revision3_quest::prepare_revision3_quest_source_inspection_v3;
use gore_story_catalog::StoryCatalogFile;
use gore_story_inventory::{
    build_base_game_inventory, BaseGameCollisionInventory,
    VerifiedRevision3QuestCollisionInspectionCapabilityV2,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

use crate::authoring_story_compiler_revision3::redact_private_paths;
use crate::authoring_story_quest_inspection_revision3::{
    build_fresh_game_inputs, revalidate_game_inputs,
};
use crate::err;
use crate::script_compile_report::{
    compile_error_parts, diagnostics_rejection, diagnostics_report_json, install_restore_label,
};

pub(super) const COMMAND: &str = "authoring_store_check_revision3_project_compiler_v1";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 8 * 1024;
const MAX_RESPONSE_BYTES: usize = 32 * 1024 * 1024;
const MAX_WIRE_BYTES: usize = MAX_PATH_BYTES * 18 + MAX_HEAD_JSON_BYTES * 2 + 8 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectCompilerWirePayload {
    expected_head_json: String,
    game_root: String,
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

#[derive(Debug)]
struct InitialSelection {
    store: WorkingProjectStore,
    expected_head: WorkingHead,
    expected_head_json: String,
    project: ProjectRevision3,
    canonical_project_json: String,
    project_seal: ContentSeal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum ManagedOwnerKind {
    NpcDraft,
    QuestDraft,
}

impl ManagedOwnerKind {
    fn expected_kind(self) -> Revision3EntityKind {
        match self {
            Self::NpcDraft => Revision3EntityKind::NpcDraft,
            Self::QuestDraft => Revision3EntityKind::QuestDraft,
        }
    }
}

#[derive(Debug, Clone)]
struct ModuleClaim {
    owner_kind: ManagedOwnerKind,
    owner_id: EntityId,
    owner_revision: u64,
    module_id: EntityId,
    module_revision: u64,
    persisted: Revision3ScriptModule,
    generated: Option<Revision3ScriptModule>,
}

#[derive(Debug)]
struct ClosedModuleGraph {
    modules: Vec<ModuleClaim>,
    quest_count: usize,
    npc_count: usize,
}

#[derive(Debug, Serialize)]
struct ModuleManifestV1<'a> {
    format: &'static str,
    schema_revision: u32,
    project_id: String,
    project_revision: u64,
    canonical_project: &'a ContentSeal,
    modules: Vec<ModuleManifestEntryV1>,
}

#[derive(Debug, Serialize)]
struct ModuleManifestEntryV1 {
    owner_kind: ManagedOwnerKind,
    owner_id: String,
    owner_revision: u64,
    module_id: String,
    module_revision: u64,
    module_namespace: String,
    module_relative_path: String,
    source: ContentSeal,
}

struct FreshGameInputs {
    catalog: StoryCatalogFile,
    shipping: Vec<u8>,
    binds: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClosingInputDisposition {
    Exact,
    Drift,
    InspectionFailed,
    NotRun,
}

impl ClosingInputDisposition {
    const fn wire_name(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Drift => "drift",
            Self::InspectionFailed => "inspection_failed",
            Self::NotRun => "not_run",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ClosingRevalidation {
    store: ClosingInputDisposition,
    game: ClosingInputDisposition,
}

impl ClosingRevalidation {
    const NOT_RUN: Self = Self {
        store: ClosingInputDisposition::NotRun,
        game: ClosingInputDisposition::NotRun,
    };

    fn is_exact(self) -> bool {
        self.store == ClosingInputDisposition::Exact && self.game == ClosingInputDisposition::Exact
    }

    fn wire_evidence(self) -> Value {
        json!({
            "store": self.store.wire_name(),
            "game": self.game.wire_name(),
        })
    }
}

pub(super) fn check_revision3_project_compiler_v1_raw(input: &str) -> Value {
    check_revision3_project_compiler_v1_inner(input).unwrap_or_else(Failure::response)
}

fn check_revision3_project_compiler_v1_inner(input: &str) -> Result<Value, Failure> {
    let payload: ProjectCompilerWirePayload = parse_exact_wire(input)?;
    validate_path(&payload.root)?;
    validate_path(&payload.game_root)?;
    let expected_head = parse_canonical_head(&payload.expected_head_json)?;
    let selection =
        open_initial_selection(&payload.root, expected_head, payload.expected_head_json)?;
    let mut graph = close_module_graph(&selection.project)?;
    let game_root = Path::new(&payload.game_root);

    // One guard spans pristine input selection, historical Quest lowering, and the sole shared
    // compiler run. This prevents deployment/install mutation from interleaving with the seal.
    let guard = acquire_compile_install_mutation(game_root).map_err(|message| {
        Failure::new(
            "AUTHORING_REVISION3_PROJECT_COMPILER_INSTALL_UNAVAILABLE",
            message,
        )
    })?;
    run_guarded_check(selection, game_root, &mut graph, guard)
}

fn run_guarded_check(
    selection: InitialSelection,
    game_root: &Path,
    graph: &mut ClosedModuleGraph,
    mut guard: InstallMutationGuard,
) -> Result<Value, Failure> {
    let (catalog, shipping, binds) = match build_fresh_game_inputs(game_root) {
        Ok(inputs) => inputs,
        Err(failure) => {
            return release_guard_failure(
                guard,
                Failure::new(map_game_input_code(failure.code), failure.message),
            );
        }
    };
    let inputs = FreshGameInputs {
        catalog,
        shipping,
        binds,
    };
    if let Err(failure) = validate_game_target(&selection.project, &inputs.catalog) {
        return release_guard_failure(guard, failure);
    }

    if graph.quest_count != 0 {
        let base_inventory =
            match build_base_game_inventory(&inputs.catalog, &inputs.shipping, &inputs.binds) {
                Ok(inventory) => inventory,
                Err(error) => {
                    return release_guard_failure(
                        guard,
                        Failure::new(
                            "AUTHORING_REVISION3_PROJECT_COMPILER_GAME_INPUT_INVALID",
                            format!("sealed game Script inputs could not be inventoried: {error}"),
                        ),
                    );
                }
            };
        if let Err(failure) =
            regenerate_quest_modules(&selection, graph, &inputs.catalog, &base_inventory)
        {
            return release_guard_failure(guard, failure);
        }
    }
    if let Err(failure) = validate_generated_modules(&selection.project, graph) {
        return release_guard_failure(guard, failure);
    }
    if let Err(failure) = sort_and_validate_compile_identities(graph) {
        return release_guard_failure(guard, failure);
    }
    let manifest = match seal_module_manifest(&selection, graph) {
        Ok(manifest) => manifest,
        Err(failure) => return release_guard_failure(guard, failure),
    };

    let game_inputs = match game_input_evidence(&inputs.catalog) {
        Ok(evidence) => evidence,
        Err(failure) => return release_guard_failure(guard, failure),
    };
    if graph.modules.is_empty() {
        // Even an empty project returns exact game-input evidence. Audit the complete Store/game
        // tuple while the one install-mutation guard is still held, then release it.
        let closing = close_revalidation(&selection, game_root, &inputs.catalog, &inputs.shipping);
        let compiler = match guard.release() {
            Ok(()) if closing.is_exact() => empty_compiler_evidence(),
            Ok(()) => return Err(map_closing_failure(closing)),
            Err(error) => {
                guard.preserve_for_manual_recovery();
                compiler_failure_evidence(
                    "COMPILE_INSTALL_GUARD_RELEASE_FAILED",
                    error,
                    Value::Null,
                    "not_started",
                    true,
                    0,
                    "recovery_retained",
                )
            }
        };
        return project_response(&selection, graph, manifest, game_inputs, compiler, closing);
    }

    let private_workspace = tempfile::Builder::new()
        .prefix("gore-project-compiler-")
        .tempdir()
        .map_err(|_| {
            Failure::new(
                "AUTHORING_REVISION3_PROJECT_COMPILER_STAGING_UNAVAILABLE",
                "native-private project compiler workspace could not be allocated",
            )
        });
    let private_workspace = match private_workspace {
        Ok(workspace) => workspace,
        Err(failure) => return release_guard_failure(guard, failure),
    };
    let overlays = match project_compile_overlays(graph) {
        Ok(overlays) => overlays,
        Err(failure) => return release_guard_failure(guard, failure),
    };
    let compiler_opts = ProjectCompilerCheckOpts {
        game_dir: game_root.to_path_buf(),
        work_dir: private_workspace.path().to_path_buf(),
        overlays,
        base_cache: inputs.shipping,
        binds_cache: inputs.binds,
    };
    let closing = Cell::new(ClosingRevalidation::NOT_RUN);
    let report = check_project_modules_with_diagnostics_report_with_guard(
        &compiler_opts,
        &DiagnosticsOptions {
            disabled: false,
            hook_dll: None,
            inject_delay: Duration::from_secs(2),
        },
        guard,
        || {
            let audited = close_revalidation(
                &selection,
                game_root,
                &inputs.catalog,
                &compiler_opts.base_cache,
            );
            closing.set(audited);
            if audited.is_exact() {
                Ok(())
            } else {
                Err("exact-current Store/game closing audit failed".to_owned())
            }
        },
    );
    let closing = closing.get();
    debug_assert_eq!(
        closing.is_exact(),
        report.closing_audit_disposition() == ProjectCompilerClosingAuditDisposition::Passed
    );
    let compiler = project_compiler_evidence(report, &[private_workspace.path()]);
    project_response(&selection, graph, manifest, game_inputs, compiler, closing)
}

/// Map an empty-project closing audit to a typed top-level failure.
///
/// With no compiler run there is no meaningful failed compiler outcome. Keeping this outside the
/// evidence response also preserves the wire invariant that zero module coverage is either clean
/// `not_needed_empty` evidence or a recovery-dominant guard-release failure. Store inspection
/// uncertainty has its own fail-closed code so the app can require a full project reopen, while a
/// game-only inspection failure remains independently retryable.
fn map_closing_failure(closing: ClosingRevalidation) -> Failure {
    match (closing.store, closing.game) {
        (ClosingInputDisposition::Drift, _) => head_conflict(),
        (ClosingInputDisposition::InspectionFailed, _) => Failure::new(
            "AUTHORING_REVISION3_PROJECT_COMPILER_CLOSING_STORE_AUDIT_FAILED",
            "the exact-current Store closing audit could not fully reopen the project",
        ),
        (ClosingInputDisposition::Exact, ClosingInputDisposition::Drift) => Failure::new(
            "AUTHORING_REVISION3_PROJECT_COMPILER_GAME_DRIFT",
            "the sealed game generation changed during project compiler checking",
        ),
        (ClosingInputDisposition::Exact, ClosingInputDisposition::InspectionFailed) => {
            Failure::new(
                "AUTHORING_REVISION3_PROJECT_COMPILER_CLOSING_GAME_AUDIT_FAILED",
                "the sealed game generation closing audit could not be completed",
            )
        }
        _ => invariant_failure(),
    }
}

fn open_initial_selection(
    root: &str,
    expected_head: WorkingHead,
    expected_head_json: String,
) -> Result<InitialSelection, Failure> {
    let store = WorkingProjectStore::open_existing(Path::new(root), ffi_store_limits())
        .map_err(map_store_error)?;
    let opened = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if opened.head != expected_head {
        return Err(head_conflict());
    }
    if serde_json::to_string(&opened.head).ok().as_deref() != Some(&expected_head_json) {
        return Err(Failure::new(
            "AUTHORING_REVISION3_PROJECT_COMPILER_HEAD_INVALID",
            "expected_head_json is not exact canonical JSON",
        ));
    }
    let canonical_project_json = opened
        .project
        .to_canonical_json()
        .map_err(|_| invariant_failure())?;
    let project_seal = seal_bytes(canonical_project_json.as_bytes());
    validate_signed_wire_project(&opened.project, &project_seal)?;
    Ok(InitialSelection {
        store,
        expected_head,
        expected_head_json,
        project: opened.project,
        canonical_project_json,
        project_seal,
    })
}

fn close_module_graph(project: &ProjectRevision3) -> Result<ClosedModuleGraph, Failure> {
    let mut claims = BTreeMap::<EntityId, ModuleClaim>::new();
    let mut quest_count = 0usize;
    let mut npc_count = 0usize;

    for (owner_id, owner_entity) in &project.entities {
        let (owner_kind, module_ref, generated) = match &owner_entity.payload {
            Revision3EntityPayload::QuestDraft(draft) => {
                quest_count = quest_count.checked_add(1).ok_or_else(limit_failure)?;
                (ManagedOwnerKind::QuestDraft, &draft.script_module, None)
            }
            Revision3EntityPayload::NpcDraft(draft) => {
                npc_count = npc_count.checked_add(1).ok_or_else(limit_failure)?;
                let owner = Revision3TypedRef::new(
                    project.project_id,
                    *owner_id,
                    Revision3EntityKind::NpcDraft,
                );
                let generated = draft.regenerate_script_module(owner).map_err(|error| {
                    Failure::new(
                        "AUTHORING_REVISION3_PROJECT_COMPILER_SOURCE_INVALID",
                        format!("NPC {owner_id} source could not be regenerated: {error}"),
                    )
                })?;
                (
                    ManagedOwnerKind::NpcDraft,
                    &draft.script_module,
                    Some(generated),
                )
            }
            _ => continue,
        };
        if module_ref.project_id != project.project_id
            || module_ref.expected_kind != Revision3EntityKind::ScriptModule
            || module_ref.id == *owner_id
        {
            return Err(graph_failure(
                "a Story draft has a foreign or mistyped ScriptModule reference",
            ));
        }
        let module_entity = project
            .entities
            .get(&module_ref.id)
            .ok_or_else(|| graph_failure("a Story draft references a missing ScriptModule"))?;
        let Revision3EntityPayload::ScriptModule(persisted) = &module_entity.payload else {
            return Err(graph_failure(
                "a Story draft reference does not target a ScriptModule",
            ));
        };
        if let Some(first) = claims.get(&module_ref.id) {
            return Err(Failure::new(
                "AUTHORING_REVISION3_PROJECT_COMPILER_MODULE_GRAPH_INVALID",
                format!(
                    "ScriptModule {} is claimed by both {} and {}",
                    module_ref.id, first.owner_id, owner_id
                ),
            ));
        }
        validate_owner_and_origin(project, *owner_id, owner_kind, module_entity, persisted)?;
        claims.insert(
            module_ref.id,
            ModuleClaim {
                owner_kind,
                owner_id: *owner_id,
                owner_revision: owner_entity.revision,
                module_id: module_ref.id,
                module_revision: module_entity.revision,
                persisted: persisted.clone(),
                generated,
            },
        );
    }

    let script_modules = project
        .entities
        .iter()
        .filter_map(|(id, entity)| {
            matches!(&entity.payload, Revision3EntityPayload::ScriptModule(_)).then_some(*id)
        })
        .collect::<BTreeSet<_>>();
    if script_modules.len() != claims.len()
        || script_modules
            .iter()
            .any(|module_id| !claims.contains_key(module_id))
    {
        return Err(graph_failure(
            "every ScriptModule must have exactly one Quest or NPC Draft owner",
        ));
    }
    let expected_count = quest_count
        .checked_add(npc_count)
        .ok_or_else(limit_failure)?;
    if claims.len() != expected_count {
        return Err(graph_failure(
            "Story draft and ScriptModule ownership counts do not close exactly",
        ));
    }
    validate_module_count(claims.len())?;
    Ok(ClosedModuleGraph {
        modules: claims.into_values().collect(),
        quest_count,
        npc_count,
    })
}

fn validate_module_count(count: usize) -> Result<(), Failure> {
    if count > MAX_PROJECT_COMPILER_CHECK_MODULES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_PROJECT_COMPILER_INPUT_LIMIT",
            format!(
                "project has {count} managed ScriptModules; maximum is {MAX_PROJECT_COMPILER_CHECK_MODULES}"
            ),
        ));
    }
    Ok(())
}

fn validate_owner_and_origin(
    project: &ProjectRevision3,
    owner_id: EntityId,
    owner_kind: ManagedOwnerKind,
    module_entity: &gore_authoring::Revision3Entity,
    module: &Revision3ScriptModule,
) -> Result<(), Failure> {
    let expected_owner =
        Revision3TypedRef::new(project.project_id, owner_id, owner_kind.expected_kind());
    if module.owner != expected_owner {
        return Err(graph_failure(
            "a ScriptModule owner does not match its one claiming Story draft",
        ));
    }
    let (generator_id, generator_version) = match owner_kind {
        ManagedOwnerKind::NpcDraft => (
            LOGICAL_NPC_CLONE_GENERATOR_ID,
            LOGICAL_NPC_CLONE_GENERATOR_VERSION,
        ),
        ManagedOwnerKind::QuestDraft => (
            REVISION3_QUEST_GENERATOR_ID,
            REVISION3_QUEST_GENERATOR_VERSION,
        ),
    };
    if module.generator_id != generator_id
        || module.generator_version != generator_version
        || !matches!(
            &module_entity.origin,
            Revision3OriginRef::Generated {
                generator_id: origin_id,
                generator_version: origin_version,
                owner,
            } if origin_id == generator_id
                && *origin_version == generator_version
                && owner == &expected_owner
        )
    {
        return Err(graph_failure(
            "a ScriptModule generator/origin does not close over its exact owner",
        ));
    }
    Ok(())
}

fn regenerate_quest_modules(
    selection: &InitialSelection,
    graph: &mut ClosedModuleGraph,
    catalog: &StoryCatalogFile,
    base_inventory: &BaseGameCollisionInventory,
) -> Result<(), Failure> {
    for module in graph
        .modules
        .iter_mut()
        .filter(|module| module.owner_kind == ManagedOwnerKind::QuestDraft)
    {
        let prepared = prepare_revision3_quest_source_inspection_v3(
            &selection.store,
            &selection.canonical_project_json,
            module.owner_id,
        )
        .map_err(map_quest_source_error)?;
        let source = prepared
            .prepare_collision_inspection_source(&selection.store)
            .map_err(map_quest_source_error)?;
        let capability = VerifiedRevision3QuestCollisionInspectionCapabilityV2::bind(
            base_inventory.clone(),
            catalog,
            source,
        )
        .map_err(|error| {
            Failure::new(
                "AUTHORING_REVISION3_PROJECT_COMPILER_SOURCE_INVALID",
                format!("Quest collision source could not be rebound exactly: {error}"),
            )
        })?;
        let plan = prepared.lower(capability).map_err(map_quest_source_error)?;
        if plan.module.quest.id != module.owner_id
            || plan.module.script_module.id != module.module_id
        {
            return Err(invariant_failure());
        }
        module.generated = Some(plan.module.generated);
    }
    Ok(())
}

fn validate_generated_modules(
    project: &ProjectRevision3,
    graph: &ClosedModuleGraph,
) -> Result<(), Failure> {
    for module in &graph.modules {
        let generated = module.generated.as_ref().ok_or_else(invariant_failure)?;
        if generated != &module.persisted
            || generated.owner.project_id != project.project_id
            || generated.owner.id != module.owner_id
            || generated.owner.expected_kind != module.owner_kind.expected_kind()
            || generated.module_namespace.is_empty()
            || generated.module_relative_path.is_empty()
            || generated.source.is_empty()
        {
            return Err(Failure::new(
                "AUTHORING_REVISION3_PROJECT_COMPILER_SOURCE_DRIFT",
                "persisted and natively regenerated project sources are not byte-exact",
            ));
        }
        let expected_path = format!("{}.as", generated.module_namespace.replace('.', "/"));
        if generated.module_relative_path != expected_path
            || generated.source_sha256
                != Sha256Digest::from_bytes(Sha256::digest(generated.source.as_bytes()).into())
        {
            return Err(invariant_failure());
        }
    }
    Ok(())
}

fn sort_and_validate_compile_identities(graph: &mut ClosedModuleGraph) -> Result<(), Failure> {
    // EntityId order is canonical, bounded, allocation-free, and independent of map insertion
    // order. Namespace/path uniqueness is checked immediately below before this order is used.
    graph.modules.sort_by_key(|module| module.module_id);
    let mut namespaces = BTreeSet::new();
    let mut relative_paths = BTreeSet::new();
    for module in &graph.modules {
        let generated = module.generated.as_ref().ok_or_else(invariant_failure)?;
        if !namespaces.insert(generated.module_namespace.to_ascii_lowercase())
            || !relative_paths.insert(generated.module_relative_path.to_ascii_lowercase())
        {
            return Err(graph_failure(
                "project ScriptModules contain a case-insensitive namespace or path collision",
            ));
        }
    }
    Ok(())
}

fn project_compile_overlays(
    graph: &ClosedModuleGraph,
) -> Result<Vec<ProjectCompileOverlay>, Failure> {
    graph
        .modules
        .iter()
        .map(|module| {
            let generated = module.generated.as_ref().ok_or_else(invariant_failure)?;
            Ok(ProjectCompileOverlay {
                module_name: generated.module_namespace.clone(),
                rel_path: generated.module_relative_path.clone(),
                source: generated.source.as_bytes().to_vec(),
            })
        })
        .collect()
}

fn seal_module_manifest(
    selection: &InitialSelection,
    graph: &ClosedModuleGraph,
) -> Result<ContentSeal, Failure> {
    let entries = graph
        .modules
        .iter()
        .map(|module| {
            let generated = module.generated.as_ref().ok_or_else(invariant_failure)?;
            Ok(ModuleManifestEntryV1 {
                owner_kind: module.owner_kind,
                owner_id: module.owner_id.to_string(),
                owner_revision: module.owner_revision,
                module_id: module.module_id.to_string(),
                module_revision: module.module_revision,
                module_namespace: generated.module_namespace.clone(),
                module_relative_path: generated.module_relative_path.clone(),
                source: seal_bytes(generated.source.as_bytes()),
            })
        })
        .collect::<Result<Vec<_>, Failure>>()?;
    let manifest = ModuleManifestV1 {
        format: "revision3_project_compiler_module_manifest",
        schema_revision: 1,
        project_id: selection.project.project_id.to_string(),
        project_revision: selection.project.revision,
        canonical_project: &selection.project_seal,
        modules: entries,
    };
    let bytes = serde_json::to_vec(&manifest).map_err(|_| invariant_failure())?;
    let seal = seal_bytes(&bytes);
    if seal.byte_len == 0 {
        return Err(invariant_failure());
    }
    signed_wire_u64(seal.byte_len)?;
    Ok(seal)
}

fn validate_game_target(
    project: &ProjectRevision3,
    catalog: &StoryCatalogFile,
) -> Result<(), Failure> {
    let executable = &catalog.generation().executable;
    if executable.byte_len != project.target.executable.byte_len
        || executable.sha256.to_string() != project.target.executable.sha256.to_string()
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_PROJECT_COMPILER_GAME_MISMATCH",
            "the selected game executable does not match the project's sealed target",
        ));
    }
    Ok(())
}

fn game_input_evidence(catalog: &StoryCatalogFile) -> Result<Value, Failure> {
    let seals = [
        &catalog.generation().executable,
        &catalog.generation().shipping_cache,
        &catalog.generation().binds_cache,
        catalog.catalog_seal(),
    ];
    for seal in seals {
        if seal.byte_len == 0 {
            return Err(invariant_failure());
        }
        signed_wire_u64(seal.byte_len)?;
    }
    Ok(json!({
        "executable": catalog.generation().executable,
        "shipping_cache": catalog.generation().shipping_cache,
        "binds_cache": catalog.generation().binds_cache,
        "story_catalog": catalog.catalog_seal(),
    }))
}

fn project_compiler_evidence(report: ProjectCompilerCheckReport, paths: &[&Path]) -> Value {
    let diagnostics_rejected = diagnostics_rejection(report.diagnostics());
    let restore = report.install_restore_disposition();
    let closing_audit = report.closing_audit_disposition();
    let recovery_required = report.recovery_required();
    let run_count = report.runner_invocations();
    let output = report.output_disposition();
    let report_invariant = run_count > 1
        || (run_count == 0 && report.diagnostics().is_some())
        || (run_count == 0
            && !recovery_required
            && (restore != InstallRestoreDisposition::NotStarted
                || output != ProjectCompilerOutputDisposition::NotCreated));
    let diagnostics = if run_count == 0 {
        Value::Null
    } else {
        report
            .diagnostics()
            .map(diagnostics_report_json)
            .unwrap_or(Value::Null)
    };
    let install_restore = if run_count == 0 && !recovery_required {
        "not_started"
    } else {
        install_restore_label(restore)
    };
    let output_disposition = match if run_count == 0 && !recovery_required {
        ProjectCompilerOutputDisposition::NotCreated
    } else {
        output
    } {
        ProjectCompilerOutputDisposition::NotCreated => "not_created",
        ProjectCompilerOutputDisposition::Discarded => "discarded",
        ProjectCompilerOutputDisposition::RecoveryRetained => "recovery_retained",
    };
    let (outcome, compile_error) = match report.outcome {
        _ if report_invariant => (
            "failed",
            json!({
                "code": "COMPILE_REPORT_INVARIANT",
                "message": "project compiler returned an internally inconsistent bounded report",
            }),
        ),
        ProjectCompilerCheckOutcome::Checked
            if diagnostics_rejected.is_none()
                && !recovery_required
                && run_count == 1
                && restore == InstallRestoreDisposition::RestoredExact
                && closing_audit == ProjectCompilerClosingAuditDisposition::Passed
                && output == ProjectCompilerOutputDisposition::Discarded =>
        {
            ("compiled_evidence_only", Value::Null)
        }
        ProjectCompilerCheckOutcome::Checked => {
            let (code, message) = diagnostics_rejected.unwrap_or((
                "COMPILE_RESTORE_INVARIANT",
                "project compiler completed without proving one run, clean output disposal, and exact installation restoration",
            ));
            ("failed", json!({"code": code, "message": message}))
        }
        ProjectCompilerCheckOutcome::Failed(error) => {
            let (code, message) = compile_error_parts(error);
            (
                "failed",
                json!({
                    "code": code,
                    "message": truncate_utf8(message, MAX_ERROR_MESSAGE_BYTES),
                }),
            )
        }
    };
    let mut evidence = json!({
        "outcome": outcome,
        "run_count": run_count,
        "compile_error": compile_error,
        "compiler_diagnostics": diagnostics,
        "install_restore": install_restore,
        "recovery_required": recovery_required,
        "output_disposition": output_disposition,
    });
    redact_private_paths(&mut evidence, paths);
    evidence
}

fn empty_compiler_evidence() -> Value {
    json!({
        "outcome": "not_needed_empty",
        "run_count": 0,
        "compile_error": Value::Null,
        "compiler_diagnostics": Value::Null,
        "install_restore": "not_started",
        "recovery_required": false,
        "output_disposition": "not_created",
    })
}

fn compiler_failure_evidence(
    code: &'static str,
    message: impl Into<String>,
    diagnostics: Value,
    install_restore: &'static str,
    recovery_required: bool,
    run_count: u8,
    output_disposition: &'static str,
) -> Value {
    json!({
        "outcome": "failed",
        "run_count": run_count,
        "compile_error": {
            "code": code,
            "message": truncate_utf8(message.into(), MAX_ERROR_MESSAGE_BYTES),
        },
        "compiler_diagnostics": diagnostics,
        "install_restore": install_restore,
        "recovery_required": recovery_required,
        "output_disposition": output_disposition,
    })
}

fn project_response(
    selection: &InitialSelection,
    graph: &ClosedModuleGraph,
    manifest: ContentSeal,
    game_inputs: Value,
    compiler: Value,
    closing: ClosingRevalidation,
) -> Result<Value, Failure> {
    let exact_current = closing.is_exact();
    let response = json!({
        "ok": true,
        "outcome": "project_compiler_check_only",
        "exact_current": exact_current,
        "head_json": selection.expected_head_json,
        "project": {
            "id": selection.project.project_id.to_string(),
            "revision": selection.project.revision,
            "seal": selection.project_seal,
        },
        "game_inputs": game_inputs,
        "coverage": {
            "script_module_count": graph.modules.len(),
            "quest_module_count": graph.quest_count,
            "npc_module_count": graph.npc_count,
            "module_manifest": manifest,
        },
        "closing_audit": closing.wire_evidence(),
        "compiler": compiler,
        "scope": "project_compiler_check_only",
        "build_status": "blocked",
        "deploy_status": "not_supported",
        "runtime_qualification": "runtime_unqualified",
        "publication_status": "not_supported",
    });
    enforce_response_budget(response)
}

fn close_revalidation(
    selection: &InitialSelection,
    game_root: &Path,
    catalog: &StoryCatalogFile,
    expected_shipping: &[u8],
) -> ClosingRevalidation {
    let store = selection
        .store
        .open_current_revision3(AssetVerification::Full)
        .ok()
        .map(|opened| {
            opened.head == selection.expected_head && opened.project == selection.project
        });
    let game = match revalidate_game_inputs(catalog, game_root, expected_shipping) {
        Ok(()) => Some(true),
        Err(failure) if failure.code.ends_with("_INPUT_CHANGED") => Some(false),
        Err(_) => None,
    };
    closing_from_parts(store, game)
}

fn closing_from_parts(store: Option<bool>, game: Option<bool>) -> ClosingRevalidation {
    fn disposition(value: Option<bool>) -> ClosingInputDisposition {
        match value {
            Some(true) => ClosingInputDisposition::Exact,
            Some(false) => ClosingInputDisposition::Drift,
            None => ClosingInputDisposition::InspectionFailed,
        }
    }
    ClosingRevalidation {
        store: disposition(store),
        game: disposition(game),
    }
}

fn release_guard_failure(guard: InstallMutationGuard, failure: Failure) -> Result<Value, Failure> {
    release_guard_failure_with(guard, failure, InstallMutationGuard::release)
}

fn release_guard_failure_with<R>(
    mut guard: InstallMutationGuard,
    failure: Failure,
    release: R,
) -> Result<Value, Failure>
where
    R: FnOnce(&mut InstallMutationGuard) -> Result<(), String>,
{
    match release(&mut guard) {
        Ok(()) => Err(failure),
        Err(release) => {
            guard.preserve_for_manual_recovery();
            Err(Failure::new(
                "AUTHORING_REVISION3_PROJECT_COMPILER_RECOVERY_REQUIRED",
                format!(
                    "{}; install guard release failed: {release}",
                    failure.message
                ),
            ))
        }
    }
}

fn parse_exact_wire<P: DeserializeOwned>(input: &str) -> Result<P, Failure> {
    if input.is_empty() || input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_PROJECT_COMPILER_INPUT_LIMIT",
            "project compiler-check request exceeds its bounded wire limit",
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != COMMAND {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn parse_canonical_head(input: &str) -> Result<WorkingHead, Failure> {
    if input.is_empty() || input.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_PROJECT_COMPILER_HEAD_INVALID",
            "expected_head_json is empty or exceeds its bounded transport limit",
        ));
    }
    let head: WorkingHead = serde_json::from_str(input).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_PROJECT_COMPILER_HEAD_INVALID",
            "expected_head_json is not one closed working head",
        )
    })?;
    if serde_json::to_string(&head).ok().as_deref() != Some(input) {
        return Err(Failure::new(
            "AUTHORING_REVISION3_PROJECT_COMPILER_HEAD_INVALID",
            "expected_head_json is not duplicate-free canonical JSON",
        ));
    }
    Ok(head)
}

fn validate_path(path: &str) -> Result<(), Failure> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\0') {
        return Err(invalid_request());
    }
    Ok(())
}

fn validate_signed_wire_project(
    project: &ProjectRevision3,
    project_seal: &ContentSeal,
) -> Result<(), Failure> {
    for value in [
        project.revision,
        project.target.executable.byte_len,
        project_seal.byte_len,
    ] {
        signed_wire_u64(value)?;
    }
    if project_seal.byte_len == 0 || project.target.executable.byte_len == 0 {
        return Err(invariant_failure());
    }
    for entity in project.entities.values() {
        signed_wire_u64(entity.revision)?;
    }
    for asset in project.asset_store.assets.values() {
        signed_wire_u64(asset.byte_len)?;
    }
    Ok(())
}

fn signed_wire_u64(value: u64) -> Result<(), Failure> {
    if value > i64::MAX as u64 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_PROJECT_COMPILER_RESPONSE_LIMIT",
            "project compiler evidence contains an integer outside the signed wire range",
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
    let bytes = serde_json::to_vec(&response).map_err(|_| invariant_failure())?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_PROJECT_COMPILER_RESPONSE_LIMIT",
            "project compiler-check response exceeds its bounded transport budget",
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

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::MissingRoot(_) => {
            "AUTHORING_REVISION3_PROJECT_COMPILER_STORE_ROOT_MISSING"
        }
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_PROJECT_COMPILER_HEAD_MISSING",
        WorkingStoreError::HeadConflict { .. } => {
            "AUTHORING_REVISION3_PROJECT_COMPILER_HEAD_CONFLICT"
        }
        WorkingStoreError::LimitExceeded { .. } => {
            "AUTHORING_REVISION3_PROJECT_COMPILER_INPUT_LIMIT"
        }
        _ => "AUTHORING_REVISION3_PROJECT_COMPILER_STORE_INVALID",
    };
    Failure::new(
        code,
        "the exact revision-3 working Store could not be opened",
    )
}

fn map_game_input_code(code: &str) -> &'static str {
    if code.contains("RECOVERY_REQUIRED") {
        "AUTHORING_REVISION3_PROJECT_COMPILER_RECOVERY_REQUIRED"
    } else if code.contains("INPUT_LIMIT") {
        "AUTHORING_REVISION3_PROJECT_COMPILER_INPUT_LIMIT"
    } else if code.contains("INPUT_CHANGED") {
        "AUTHORING_REVISION3_PROJECT_COMPILER_GAME_DRIFT"
    } else if code.contains("UNSUPPORTED_GENERATION") {
        "AUTHORING_REVISION3_PROJECT_COMPILER_UNSUPPORTED_GENERATION"
    } else {
        "AUTHORING_REVISION3_PROJECT_COMPILER_GAME_INPUT_UNAVAILABLE"
    }
}

fn map_quest_source_error(
    error: gore_story_build::revision3_quest::Revision3QuestInspectionError,
) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_PROJECT_COMPILER_SOURCE_INVALID",
        format!("Quest source could not be regenerated from the exact Store snapshot: {error}"),
    )
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_PROJECT_COMPILER_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and exactly expected_head_json, game_root, and root",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_PROJECT_COMPILER_HEAD_CONFLICT",
        "the published revision-3 head changed or differs from the caller's exact head",
    )
}

fn graph_failure(message: impl Into<String>) -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_PROJECT_COMPILER_MODULE_GRAPH_INVALID",
        message,
    )
}

fn limit_failure() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_PROJECT_COMPILER_INPUT_LIMIT",
        "project compiler-check counts exceed their bounded integer range",
    )
}

fn invariant_failure() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_PROJECT_COMPILER_INVARIANT",
        "native project compiler-check invariants were not satisfied",
    )
}

fn truncate_utf8(mut text: String, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text;
    }
    const SUFFIX: &str = "...";
    let mut end = max_bytes.saturating_sub(SUFFIX.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text.push_str(SUFFIX);
    text
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;

    use gore_as::compile::acquire_compile_install_mutation_with_stated_game_process;
    use gore_authoring::{
        AssetMeta, AssetStoreIndex, FormatV2, GameGenerationAnchor, NpcParentClassInput, ProjectId,
        ProjectMeta, QuestCollisionArtifactRef, QuestCollisionCatalogInput, Revision3Entity,
        Revision3NpcDraft, Revision3NpcDraftInput, Revision3QuestDraft, Revision3QuestDraftInput,
        Revision3QuestGiverInput, Revision3QuestParentInput, SchemaRevisionV3, ScriptModuleStatus,
        WorkingStoreFormat, QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2,
        QUEST_COLLISION_CATALOG_LAYER_V2,
    };
    use tempfile::TempDir;

    use super::*;

    const NPC_BYTE: u8 = 0x31;
    const MODULE_BYTE: u8 = 0x32;

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

    fn empty_project(revision: u64) -> ProjectRevision3 {
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([0x21; 16]),
            revision,
            meta: ProjectMeta {
                name: "Project compiler check".to_owned(),
                version: "1.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: target(),
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn parent(value: u8, runtime_class: &str) -> NpcParentClassInput {
        NpcParentClassInput {
            generation: target(),
            source_seal: content_seal(value, 4_096),
            catalog_layer: "base-game.g1r.npc-parents.v1".to_owned(),
            canonical_selector: runtime_class.to_owned(),
            runtime_class: runtime_class.to_owned(),
        }
    }

    fn npc_project(revision: u64) -> ProjectRevision3 {
        let mut project = empty_project(revision);
        let npc_id = EntityId::from_bytes([NPC_BYTE; 16]);
        let module_id = EntityId::from_bytes([MODULE_BYTE; 16]);
        let owner =
            Revision3TypedRef::new(project.project_id, npc_id, Revision3EntityKind::NpcDraft);
        let draft = Revision3NpcDraft {
            generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
            generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
            input: Revision3NpcDraftInput {
                target: target(),
                module_namespace: "GoreMods.Npcs.ProjectCompilerCheck".to_owned(),
                unique_name: "GORE_PROJECT_COMPILER_CHECK".to_owned(),
                parent_character_definition: parent(
                    2,
                    "UCharacterDefinition_Human_OM_GRD_Asghan_263",
                ),
                parent_ai_agent_config: parent(3, "UAIAgentConfig_Human_OM_GRD_Asghan_263"),
                parent_spawn_definition: parent(4, "USpawnAIAgentDefinition_OM_GRD_Asghan_263"),
            },
            script_module: Revision3TypedRef::new(
                project.project_id,
                module_id,
                Revision3EntityKind::ScriptModule,
            ),
            greetings: Vec::new(),
        };
        let module = draft.regenerate_script_module(owner.clone()).unwrap();
        project.entities.insert(
            npc_id,
            Revision3Entity {
                id: npc_id,
                display_name: "Project compiler NPC".to_owned(),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: draft.input.unique_name.clone(),
                },
                revision: 2,
                payload: Revision3EntityPayload::NpcDraft(draft),
            },
        );
        project.entities.insert(
            module_id,
            Revision3Entity {
                id: module_id,
                display_name: "Project compiler source".to_owned(),
                origin: Revision3OriginRef::Generated {
                    generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
                    generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
                    owner,
                },
                revision: 3,
                payload: Revision3EntityPayload::ScriptModule(module),
            },
        );
        project
    }

    fn quest_project(revision: u64) -> (ProjectRevision3, Revision3ScriptModule) {
        let mut project = empty_project(revision);
        let quest_id = EntityId::from_bytes([0x51; 16]);
        let module_id = EntityId::from_bytes([0x52; 16]);
        let owner = Revision3TypedRef::new(
            project.project_id,
            quest_id,
            Revision3EntityKind::QuestDraft,
        );
        let collision_source = content_seal(0x55, 4_096);
        let draft = Revision3QuestDraft {
            generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
            generator_version: REVISION3_QUEST_GENERATOR_VERSION,
            input: Revision3QuestDraftInput {
                target: target(),
                quest_id,
                module_namespace: "GoreMods.Quests.ProjectCompilerCheck".to_owned(),
                technical_id: "GORE_PROJECT_COMPILER_QUEST".to_owned(),
                text_helper: "GoreProjectCompilerQuestText".to_owned(),
                parent_quest: Revision3QuestParentInput {
                    generation: target(),
                    source_seal: content_seal(0x53, 856),
                    catalog_layer: "base-game.g1r.scripts".to_owned(),
                    canonical_selector: "CatalogQuestParent".to_owned(),
                    runtime_class: "UQuest_SwampCamp_SCCHAPTER2".to_owned(),
                },
                giver: Revision3QuestGiverInput {
                    generation: target(),
                    source_seal: content_seal(0x54, 856),
                    catalog_layer: "base-game.g1r.scripts".to_owned(),
                    canonical_selector: "CatalogAsghan".to_owned(),
                    runtime_unique_name: "OM_GRD_Asghan_263".to_owned(),
                },
                title: "Project compiler Quest".to_owned(),
                description: "Prove the whole project compiles together.".to_owned(),
                objective_title: "Report to Asghan".to_owned(),
                additional_objective_titles: Vec::new(),
                transition_plan: Box::new(
                    gore_authoring::QuestTransitionPlanV1::default_for_objectives(1).unwrap(),
                ),
                collision_catalog: QuestCollisionArtifactRef {
                    generation: target(),
                    catalog_layer: QUEST_COLLISION_CATALOG_LAYER_V2.to_owned(),
                    artifact: content_seal(0x56, 4_096),
                    source_seal: collision_source.clone(),
                    basis_snapshot: content_seal(0x57, 800),
                },
            },
            script_module: Revision3TypedRef::new(
                project.project_id,
                module_id,
                Revision3EntityKind::ScriptModule,
            ),
            transcript: Vec::new(),
        };
        let generated = gore_authoring::regenerate_revision3_quest_module(
            &draft,
            QuestCollisionCatalogInput {
                generation: target(),
                source_seal: collision_source,
                catalog_layer: QUEST_COLLISION_CATALOG_LAYER_V2.to_owned(),
                modules: BTreeSet::new(),
                relative_paths: BTreeSet::new(),
                symbols: BTreeSet::new(),
            },
        )
        .unwrap();
        project.asset_store.assets.insert(
            draft.input.collision_catalog.artifact.sha256.clone(),
            AssetMeta {
                byte_len: draft.input.collision_catalog.artifact.byte_len,
                media_type: QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2.to_owned(),
            },
        );
        project.entities.insert(
            quest_id,
            Revision3Entity {
                id: quest_id,
                display_name: "Project compiler Quest".to_owned(),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: draft.input.technical_id.clone(),
                },
                revision: 4,
                payload: Revision3EntityPayload::QuestDraft(draft),
            },
        );
        project.entities.insert(
            module_id,
            Revision3Entity {
                id: module_id,
                display_name: "Project compiler Quest source".to_owned(),
                origin: Revision3OriginRef::Generated {
                    generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
                    generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                    owner,
                },
                revision: 5,
                payload: Revision3EntityPayload::ScriptModule(generated.clone()),
            },
        );
        (project, generated)
    }

    fn mixed_story_project(revision: u64) -> (ProjectRevision3, Revision3ScriptModule) {
        let (mut project, quest_generated) = quest_project(revision);
        project.entities.extend(npc_project(revision).entities);
        (project, quest_generated)
    }

    fn published_store(project: &ProjectRevision3) -> (TempDir, String) {
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let prepared = store.prepare_revision3_checkpoint(None, project).unwrap();
        fs::write(temp.path().join("gore-project.json"), &prepared.head_bytes).unwrap();
        (temp, String::from_utf8(prepared.head_bytes).unwrap())
    }

    fn wire(payload: Value) -> String {
        json!({"command": COMMAND, "payload": payload}).to_string()
    }

    fn valid_shape() -> Value {
        json!({
            "expected_head_json": serde_json::to_string(&WorkingHead {
                store_format: WorkingStoreFormat,
                snapshot: content_seal(9, 123),
            }).unwrap(),
            "game_root": "C:/missing-game",
            "root": "C:/missing-store",
        })
    }

    #[test]
    fn exact_wire_rejects_duplicate_unknown_and_forged_authority() {
        let parsed: ProjectCompilerWirePayload = parse_exact_wire(&wire(valid_shape())).unwrap();
        assert_eq!(parsed.game_root, "C:/missing-game");

        for (field, value) in [
            ("source", json!("forged AngelScript")),
            ("sources", json!([])),
            ("module", json!("GoreMods.Forged")),
            ("module_name", json!("GoreMods.Forged")),
            ("rel_path", json!("GoreMods/Forged.as")),
            ("work_dir", json!("C:/caller-work")),
            ("output", json!("C:/artifact")),
            ("build", json!(true)),
            ("deploy", json!(true)),
            ("allow_new_symbols", json!(false)),
        ] {
            let mut payload = valid_shape();
            payload[field] = value;
            assert_eq!(
                check_revision3_project_compiler_v1_raw(&wire(payload))["error"]["code"],
                "AUTHORING_REVISION3_PROJECT_COMPILER_REQUEST_INVALID",
                "accepted forged field {field}"
            );
        }

        let payload = valid_shape();
        let head = serde_json::to_string(&payload["expected_head_json"]).unwrap();
        let game = serde_json::to_string(&payload["game_root"]).unwrap();
        let root = serde_json::to_string(&payload["root"]).unwrap();
        let duplicate = format!(
            "{{\"command\":\"{COMMAND}\",\"payload\":{{\"expected_head_json\":{head},\"game_root\":{game},\"game_root\":{game},\"root\":{root}}}}}"
        );
        assert_eq!(
            check_revision3_project_compiler_v1_raw(&duplicate)["error"]["code"],
            "AUTHORING_REVISION3_PROJECT_COMPILER_REQUEST_INVALID"
        );
        let duplicate_command = format!(
            "{{\"command\":\"{COMMAND}\",\"command\":\"{COMMAND}\",\"payload\":{}}}",
            serde_json::to_string(&payload).unwrap()
        );
        assert_eq!(
            check_revision3_project_compiler_v1_raw(&duplicate_command)["error"]["code"],
            "AUTHORING_REVISION3_PROJECT_COMPILER_REQUEST_INVALID"
        );
    }

    #[test]
    fn wire_and_head_limits_fail_before_store_or_game_access() {
        let oversized = "x".repeat(MAX_WIRE_BYTES + 1);
        assert_eq!(
            check_revision3_project_compiler_v1_raw(&oversized)["error"]["code"],
            "AUTHORING_REVISION3_PROJECT_COMPILER_INPUT_LIMIT"
        );
        let canonical = valid_shape()["expected_head_json"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(parse_canonical_head(&canonical).is_ok());
        assert_eq!(
            parse_canonical_head(&format!(" {canonical}"))
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_PROJECT_COMPILER_HEAD_INVALID"
        );
        let duplicate = canonical.replacen("{", "{\"store_format\":1,", 1);
        assert_eq!(
            parse_canonical_head(&duplicate).unwrap_err().code,
            "AUTHORING_REVISION3_PROJECT_COMPILER_HEAD_INVALID"
        );
        assert_eq!(
            parse_canonical_head(&"x".repeat(MAX_HEAD_JSON_BYTES + 1))
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_PROJECT_COMPILER_HEAD_INVALID"
        );
    }

    #[test]
    fn full_store_open_binds_exact_head_project_and_is_read_only() {
        let project = npc_project(7);
        let (temp, head_json) = published_store(&project);
        let before = fs::read(temp.path().join("gore-project.json")).unwrap();
        let selection = open_initial_selection(
            temp.path().to_str().unwrap(),
            serde_json::from_str(&head_json).unwrap(),
            head_json.clone(),
        )
        .unwrap();
        assert_eq!(selection.expected_head_json, head_json);
        assert_eq!(selection.project, project);
        assert_eq!(
            selection.project_seal,
            seal_bytes(selection.canonical_project_json.as_bytes())
        );
        assert_eq!(
            fs::read(temp.path().join("gore-project.json")).unwrap(),
            before
        );

        let mut stale: WorkingHead = serde_json::from_str(&head_json).unwrap();
        stale.snapshot.sha256 = digest(0xfe);
        assert_eq!(
            open_initial_selection(
                temp.path().to_str().unwrap(),
                stale.clone(),
                serde_json::to_string(&stale).unwrap(),
            )
            .unwrap_err()
            .code,
            "AUTHORING_REVISION3_PROJECT_COMPILER_HEAD_CONFLICT"
        );
    }

    #[test]
    fn empty_and_npc_graphs_close_with_exact_native_regeneration() {
        let empty = close_module_graph(&empty_project(1)).unwrap();
        assert!(empty.modules.is_empty());
        assert_eq!(empty.quest_count, 0);
        assert_eq!(empty.npc_count, 0);

        let project = npc_project(2);
        let graph = close_module_graph(&project).unwrap();
        assert_eq!(graph.modules.len(), 1);
        assert_eq!(graph.quest_count, 0);
        assert_eq!(graph.npc_count, 1);
        assert_eq!(graph.modules[0].owner_kind, ManagedOwnerKind::NpcDraft);
        assert_eq!(
            graph.modules[0].generated,
            Some(graph.modules[0].persisted.clone())
        );
        validate_generated_modules(&project, &graph).unwrap();
    }

    #[test]
    fn quest_graph_closes_and_accepts_only_byte_exact_native_regeneration() {
        let (project, generated) = quest_project(3);
        let mut graph = close_module_graph(&project).unwrap();
        assert_eq!(graph.modules.len(), 1);
        assert_eq!(graph.quest_count, 1);
        assert_eq!(graph.npc_count, 0);
        assert_eq!(graph.modules[0].owner_kind, ManagedOwnerKind::QuestDraft);
        assert!(graph.modules[0].generated.is_none());
        graph.modules[0].generated = Some(generated.clone());
        validate_generated_modules(&project, &graph).unwrap();

        let mut drift = generated;
        drift.source.push_str("// not persisted\n");
        drift.source_sha256 =
            Sha256Digest::from_bytes(Sha256::digest(drift.source.as_bytes()).into());
        graph.modules[0].generated = Some(drift);
        assert_eq!(
            validate_generated_modules(&project, &graph)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_PROJECT_COMPILER_SOURCE_DRIFT"
        );
    }

    #[test]
    fn mixed_quest_npc_graph_seals_one_complete_shared_overlay_batch() {
        let (project, quest_generated) = mixed_story_project(4);
        let temp = TempDir::new().unwrap();
        let canonical_project_json = serde_json::to_string(&project).unwrap();
        let project_seal = seal_bytes(canonical_project_json.as_bytes());
        let expected_head = WorkingHead {
            store_format: WorkingStoreFormat,
            snapshot: project_seal.clone(),
        };
        let selection = InitialSelection {
            store: WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap(),
            expected_head_json: serde_json::to_string(&expected_head).unwrap(),
            expected_head,
            project,
            canonical_project_json,
            project_seal,
        };
        let mut graph = close_module_graph(&selection.project).unwrap();
        assert_eq!(graph.quest_count, 1);
        assert_eq!(graph.npc_count, 1);
        assert_eq!(graph.modules.len(), 2);
        graph
            .modules
            .iter_mut()
            .find(|module| module.owner_kind == ManagedOwnerKind::QuestDraft)
            .unwrap()
            .generated = Some(quest_generated);
        validate_generated_modules(&selection.project, &graph).unwrap();
        sort_and_validate_compile_identities(&mut graph).unwrap();

        let overlays = project_compile_overlays(&graph).unwrap();
        assert_eq!(overlays.len(), 2);
        assert_eq!(
            overlays
                .iter()
                .map(|overlay| overlay.module_name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "GoreMods.Npcs.ProjectCompilerCheck",
                "GoreMods.Quests.ProjectCompilerCheck",
            ]
        );
        assert!(overlays.iter().all(|overlay| !overlay.source.is_empty()));
        assert!(seal_module_manifest(&selection, &graph).unwrap().byte_len > 0);
    }

    #[test]
    fn graph_rejects_orphan_foreign_and_shared_script_modules() {
        let mut orphan = empty_project(3);
        let owner_id = EntityId::from_bytes([0x44; 16]);
        let module_id = EntityId::from_bytes([0x45; 16]);
        let owner =
            Revision3TypedRef::new(orphan.project_id, owner_id, Revision3EntityKind::QuestDraft);
        orphan.entities.insert(
            module_id,
            Revision3Entity {
                id: module_id,
                display_name: "Orphan Quest source".to_owned(),
                origin: Revision3OriginRef::Generated {
                    generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
                    generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                    owner: owner.clone(),
                },
                revision: 1,
                payload: Revision3EntityPayload::ScriptModule(Revision3ScriptModule {
                    generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
                    generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                    owner,
                    module_namespace: "GoreMods.Quests.Orphan".to_owned(),
                    module_relative_path: "GoreMods/Quests/Orphan.as".to_owned(),
                    source: "class GORE_ORPHAN {}\n".to_owned(),
                    source_sha256: Sha256Digest::from_bytes(
                        Sha256::digest(b"class GORE_ORPHAN {}\n").into(),
                    ),
                    input_fingerprint: digest(8),
                    status: ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED,
                }),
            },
        );
        assert_eq!(
            close_module_graph(&orphan).unwrap_err().code,
            "AUTHORING_REVISION3_PROJECT_COMPILER_MODULE_GRAPH_INVALID"
        );

        let mut foreign = npc_project(4);
        let Revision3EntityPayload::NpcDraft(draft) = &mut foreign
            .entities
            .get_mut(&EntityId::from_bytes([NPC_BYTE; 16]))
            .unwrap()
            .payload
        else {
            unreachable!()
        };
        draft.script_module.project_id = ProjectId::from_bytes([0xee; 16]);
        assert_eq!(
            close_module_graph(&foreign).unwrap_err().code,
            "AUTHORING_REVISION3_PROJECT_COMPILER_MODULE_GRAPH_INVALID"
        );

        let mut shared = npc_project(5);
        let first_id = EntityId::from_bytes([NPC_BYTE; 16]);
        let second_id = EntityId::from_bytes([0x35; 16]);
        let mut second = shared.entities[&first_id].clone();
        second.id = second_id;
        second.revision += 1;
        shared.entities.insert(second_id, second);
        assert_eq!(
            close_module_graph(&shared).unwrap_err().code,
            "AUTHORING_REVISION3_PROJECT_COMPILER_MODULE_GRAPH_INVALID"
        );
    }

    #[test]
    fn persisted_source_drift_and_compile_identity_collisions_fail_closed() {
        let mut project = npc_project(6);
        let module_id = EntityId::from_bytes([MODULE_BYTE; 16]);
        let Revision3EntityPayload::ScriptModule(module) =
            &mut project.entities.get_mut(&module_id).unwrap().payload
        else {
            unreachable!()
        };
        module.source.push_str("// drift\n");
        module.source_sha256 =
            Sha256Digest::from_bytes(Sha256::digest(module.source.as_bytes()).into());
        let graph = close_module_graph(&project).unwrap();
        assert_eq!(
            validate_generated_modules(&project, &graph)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_PROJECT_COMPILER_SOURCE_DRIFT"
        );

        let exact = npc_project(7);
        let mut graph = close_module_graph(&exact).unwrap();
        let mut duplicate = graph.modules[0].clone();
        duplicate.module_id = EntityId::from_bytes([0x39; 16]);
        graph.modules.push(duplicate);
        assert_eq!(
            sort_and_validate_compile_identities(&mut graph)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_PROJECT_COMPILER_MODULE_GRAPH_INVALID"
        );
    }

    #[test]
    fn module_count_is_rejected_before_guard_or_compiler_authority() {
        assert!(validate_module_count(MAX_PROJECT_COMPILER_CHECK_MODULES).is_ok());
        assert_eq!(
            validate_module_count(MAX_PROJECT_COMPILER_CHECK_MODULES + 1)
                .unwrap_err()
                .code,
            "AUTHORING_REVISION3_PROJECT_COMPILER_INPUT_LIMIT"
        );
    }

    #[test]
    fn module_manifest_is_deterministic_and_revision_sensitive() {
        let project = npc_project(8);
        let (temp, head_json) = published_store(&project);
        let selection = open_initial_selection(
            temp.path().to_str().unwrap(),
            serde_json::from_str(&head_json).unwrap(),
            head_json,
        )
        .unwrap();
        let mut graph = close_module_graph(&selection.project).unwrap();
        validate_generated_modules(&selection.project, &graph).unwrap();
        sort_and_validate_compile_identities(&mut graph).unwrap();
        let first = seal_module_manifest(&selection, &graph).unwrap();
        graph.modules.reverse();
        sort_and_validate_compile_identities(&mut graph).unwrap();
        assert_eq!(seal_module_manifest(&selection, &graph).unwrap(), first);
        graph.modules[0].module_revision += 1;
        assert_ne!(seal_module_manifest(&selection, &graph).unwrap(), first);
    }

    #[test]
    fn response_exposes_only_evidence_and_no_source_or_build_authority() {
        let project = empty_project(9);
        let (temp, head_json) = published_store(&project);
        let selection = open_initial_selection(
            temp.path().to_str().unwrap(),
            serde_json::from_str(&head_json).unwrap(),
            head_json.clone(),
        )
        .unwrap();
        let graph = close_module_graph(&selection.project).unwrap();
        let manifest = seal_module_manifest(&selection, &graph).unwrap();
        let response = project_response(
            &selection,
            &graph,
            manifest,
            json!({
                "executable": content_seal(1, 1),
                "shipping_cache": content_seal(2, 2),
                "binds_cache": content_seal(3, 3),
                "story_catalog": content_seal(4, 4),
            }),
            empty_compiler_evidence(),
            closing_from_parts(Some(true), Some(true)),
        )
        .unwrap();
        assert_eq!(response["ok"], true);
        assert_eq!(response["outcome"], "project_compiler_check_only");
        assert_eq!(response["exact_current"], true);
        assert_eq!(response["head_json"], head_json);
        assert_eq!(response["coverage"]["script_module_count"], 0);
        assert_eq!(response["compiler"]["outcome"], "not_needed_empty");
        assert_eq!(response["compiler"]["run_count"], 0);
        assert_eq!(response["compiler"]["output_disposition"], "not_created");
        assert_eq!(response["closing_audit"]["store"], "exact");
        assert_eq!(response["closing_audit"]["game"], "exact");
        assert_eq!(response["scope"], "project_compiler_check_only");
        assert_eq!(response["build_status"], "blocked");
        assert_eq!(response["deploy_status"], "not_supported");
        assert_eq!(response["runtime_qualification"], "runtime_unqualified");
        assert_eq!(response["publication_status"], "not_supported");
        let encoded = response.to_string();
        for forbidden in [
            "module_namespace",
            "module_relative_path",
            "source_sha256",
            "artifact_path",
            "mini_path",
            "build_ready",
            "deploy_ready",
        ] {
            assert!(!encoded.contains(forbidden), "leaked {forbidden}");
        }
    }

    #[test]
    fn closing_exactness_is_orthogonal_to_private_recovery() {
        let project = empty_project(9);
        let (temp, head_json) = published_store(&project);
        let selection = open_initial_selection(
            temp.path().to_str().unwrap(),
            serde_json::from_str(&head_json).unwrap(),
            head_json,
        )
        .unwrap();
        let graph = close_module_graph(&selection.project).unwrap();
        let manifest = seal_module_manifest(&selection, &graph).unwrap();
        let response = project_response(
            &selection,
            &graph,
            manifest,
            json!({
                "executable": content_seal(1, 1),
                "shipping_cache": content_seal(2, 2),
                "binds_cache": content_seal(3, 3),
                "story_catalog": content_seal(4, 4),
            }),
            compiler_failure_evidence(
                "COMPILE_INSTALL_RECOVERY_REQUIRED",
                "private compiler recovery remains",
                Value::Null,
                "not_started",
                true,
                0,
                "recovery_retained",
            ),
            closing_from_parts(Some(true), Some(true)),
        )
        .unwrap();

        assert_eq!(response["exact_current"], true);
        assert_eq!(response["compiler"]["recovery_required"], true);
        assert_eq!(response["closing_audit"]["store"], "exact");
        assert_eq!(response["closing_audit"]["game"], "exact");
    }

    #[test]
    fn release_failure_dominates_post_guard_preflight_failure() {
        // The stated answer is what keeps this test about which failure dominates the other.
        // `gore-ffi` links `gore-as` compiled without `cfg(test)`, so the seam its own fixtures use
        // is out of reach here and the real process inspection would answer instead — passing or
        // failing on whether the developer happens to have Gothic open while the suite runs.
        let temp = TempDir::new().unwrap();
        let game = temp.path().join("game");
        fs::create_dir_all(&game).unwrap();
        let guard =
            acquire_compile_install_mutation_with_stated_game_process(&game, || Ok(false)).unwrap();
        let failure = Failure::new(
            "AUTHORING_REVISION3_PROJECT_COMPILER_SOURCE_DRIFT",
            "injected primary source failure",
        );
        let result = release_guard_failure_with(guard, failure, |_| {
            Err("injected release uncertainty".to_owned())
        })
        .unwrap_err();
        assert_eq!(
            result.code,
            "AUTHORING_REVISION3_PROJECT_COMPILER_RECOVERY_REQUIRED"
        );
        assert!(result.message.contains("injected primary source failure"));
        assert!(result.message.contains("injected release uncertainty"));
    }

    #[test]
    fn closing_state_is_exact_only_when_store_and_game_are_exact() {
        assert_eq!(
            closing_from_parts(Some(true), Some(true)),
            ClosingRevalidation {
                store: ClosingInputDisposition::Exact,
                game: ClosingInputDisposition::Exact,
            }
        );
        assert_eq!(
            closing_from_parts(Some(false), Some(true)),
            ClosingRevalidation {
                store: ClosingInputDisposition::Drift,
                game: ClosingInputDisposition::Exact,
            }
        );
        assert_eq!(
            closing_from_parts(Some(true), Some(false)),
            ClosingRevalidation {
                store: ClosingInputDisposition::Exact,
                game: ClosingInputDisposition::Drift,
            }
        );
        assert_eq!(
            closing_from_parts(Some(false), Some(false)),
            ClosingRevalidation {
                store: ClosingInputDisposition::Drift,
                game: ClosingInputDisposition::Drift,
            }
        );
        assert_eq!(
            closing_from_parts(None, Some(true)),
            ClosingRevalidation {
                store: ClosingInputDisposition::InspectionFailed,
                game: ClosingInputDisposition::Exact,
            }
        );
        assert_eq!(
            map_closing_failure(closing_from_parts(Some(false), Some(true))).code,
            "AUTHORING_REVISION3_PROJECT_COMPILER_HEAD_CONFLICT"
        );
        assert_eq!(
            map_closing_failure(closing_from_parts(Some(true), Some(false))).code,
            "AUTHORING_REVISION3_PROJECT_COMPILER_GAME_DRIFT"
        );
        assert_eq!(
            map_closing_failure(closing_from_parts(None, Some(true))).code,
            "AUTHORING_REVISION3_PROJECT_COMPILER_CLOSING_STORE_AUDIT_FAILED"
        );
        assert_eq!(
            map_closing_failure(closing_from_parts(Some(true), None)).code,
            "AUTHORING_REVISION3_PROJECT_COMPILER_CLOSING_GAME_AUDIT_FAILED"
        );
    }

    #[test]
    fn dispatcher_registers_the_closed_raw_route() {
        let response: Value =
            serde_json::from_str(&crate::execute_json(&wire(valid_shape()))).unwrap();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_PROJECT_COMPILER_STORE_ROOT_MISSING"
        );
    }

    #[test]
    #[ignore = "requires a supported closed Gothic 1 Remake installation and compiler runtime"]
    fn supported_game_checks_one_npc_in_exactly_one_shared_compiler_run() {
        let game_root = std::env::var("GORE_GAME_ROOT")
            .expect("set GORE_GAME_ROOT to the selected game installation");
        let (catalog, _, _) = build_fresh_game_inputs(Path::new(&game_root)).unwrap();
        let live_target = GameGenerationAnchor {
            executable: ContentSeal {
                byte_len: catalog.generation().executable.byte_len,
                sha256: Sha256Digest::from_bytes(
                    *catalog.generation().executable.sha256.as_bytes(),
                ),
            },
        };
        let mut project = npc_project(10);
        project.target = live_target.clone();
        let project_id = project.project_id;
        let npc_id = EntityId::from_bytes([NPC_BYTE; 16]);
        let module_id = EntityId::from_bytes([MODULE_BYTE; 16]);
        let generated = {
            let Revision3EntityPayload::NpcDraft(draft) =
                &mut project.entities.get_mut(&npc_id).unwrap().payload
            else {
                unreachable!()
            };
            draft.input.target = live_target.clone();
            draft.input.parent_character_definition.generation = live_target.clone();
            draft.input.parent_ai_agent_config.generation = live_target.clone();
            draft.input.parent_spawn_definition.generation = live_target;
            draft
                .regenerate_script_module(Revision3TypedRef::new(
                    project_id,
                    npc_id,
                    Revision3EntityKind::NpcDraft,
                ))
                .unwrap()
        };
        project.entities.get_mut(&module_id).unwrap().payload =
            Revision3EntityPayload::ScriptModule(generated);
        let (store, head_json) = published_store(&project);
        let response = check_revision3_project_compiler_v1_raw(&wire(json!({
            "expected_head_json": head_json,
            "game_root": game_root,
            "root": store.path().to_str().unwrap(),
        })));
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["exact_current"], true);
        assert_eq!(response["coverage"]["script_module_count"], 1);
        assert_eq!(response["coverage"]["npc_module_count"], 1);
        assert_eq!(response["compiler"]["outcome"], "compiled_evidence_only");
        assert_eq!(response["compiler"]["run_count"], 1);
        assert_eq!(response["compiler"]["output_disposition"], "discarded");
    }
}
