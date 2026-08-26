//! Exact-current, evidence-only compiler checks for managed revision-3 Quests and NPCs.
//!
//! Both routes accept only a Store/head selection, one entity id, and a selected game installation.
//! Native code reconstructs the persisted module through the existing sealed inspection primitives,
//! owns an unreported private compiler workspace, fixes policy to `add` with new symbols enabled,
//! and discards the generated mini-cache. The response is evidence, never build/deploy authority.

use std::path::Path;
use std::time::Duration;

use gore_as::compile::{
    acquire_compile_install_mutation, compile_module_with_backend_v1,
    compile_module_with_backend_v1_with_guard_and_target,
    compile_module_with_diagnostics_report_with_guard, CompileModuleReportOutcome, CompileOpts,
    CompilerBackendModeV1, CompilerBackendNameV1, InstallMutationGuard,
};
use gore_as::diagnostics::DiagnosticsOptions;
use gore_authoring::{
    AssetVerification, ContentSeal, EntityId, ProjectRevision3, Revision3EntityKind,
    Revision3EntityPayload, Revision3ScriptModule, Sha256Digest, WorkingHead, WorkingProjectStore,
    WorkingStoreError, WorkingStoreLimits, MAX_PROJECT_JSON_BYTES,
};
use gore_story_build::revision3_npc::Revision3NpcSourceInspectionPlanV1;
use gore_story_build::revision3_quest::Revision3QuestSourceInspectionPlanV3;
use gore_story_catalog::StoryCatalogFile;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

use crate::authoring_story_quest_inspection_revision3::{
    build_fresh_game_inputs, revalidate_game_inputs,
};
use crate::err;
use crate::script_compile_report::{
    discard_owned_compiled_mini, discard_owned_failed_compiled_mini, install_guard_failure,
    report_response, report_response_with_policy, OwnedCompileStaging,
};
use crate::standalone_compiler_package::{
    backend_evidence, backend_evidence_with_package, bundle_absent_fallback_reason,
    package_unavailable_fallback_reason, resolve_product_standalone_compiler_for_game_v1,
    CompilerBackendWireV2, ResolvedProductStandaloneCompilerV1, BUNDLE_ABSENT_DETAIL,
};

pub(super) const QUEST_COMMAND: &str = "authoring_store_check_revision3_quest_compiler_v1";
pub(super) const NPC_COMMAND: &str = "authoring_store_check_revision3_npc_compiler_v1";
pub(super) const QUEST_COMMAND_V2: &str = "authoring_store_check_revision3_quest_compiler_v2";
pub(super) const NPC_COMMAND_V2: &str = "authoring_store_check_revision3_npc_compiler_v2";

const MAX_PATH_BYTES: usize = 32 * 1024;
const MAX_HEAD_JSON_BYTES: usize = 64 * 1024;
const ENTITY_ID_BYTES: usize = 32;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_RESPONSE_BYTES: usize = 32 * 1024 * 1024;
const PRIVATE_PATH_REDACTION: &str = "<native-private compiler path>";
const MAX_WIRE_BYTES: usize =
    MAX_PATH_BYTES * 24 + MAX_HEAD_JSON_BYTES * 2 + ENTITY_ID_BYTES * 2 + 8 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactWireRequest<P> {
    command: String,
    payload: P,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QuestWirePayload {
    expected_head_json: String,
    game_root: String,
    quest_id: String,
    root: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NpcWirePayload {
    expected_head_json: String,
    game_root: String,
    npc_id: String,
    root: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QuestWirePayloadV2 {
    compiler_backend: CompilerBackendWireV2,
    expected_head_json: String,
    game_root: String,
    quest_id: String,
    root: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NpcWirePayloadV2 {
    compiler_backend: CompilerBackendWireV2,
    expected_head_json: String,
    game_root: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManagedEntityKind {
    Quest,
    Npc,
}

impl ManagedEntityKind {
    fn label(self) -> &'static str {
        match self {
            Self::Quest => "quest_draft",
            Self::Npc => "npc_draft",
        }
    }

    fn expected_kind(self) -> Revision3EntityKind {
        match self {
            Self::Quest => Revision3EntityKind::QuestDraft,
            Self::Npc => Revision3EntityKind::NpcDraft,
        }
    }
}

struct InitialSelection {
    store: WorkingProjectStore,
    expected_head: WorkingHead,
    expected_head_json: String,
    project: ProjectRevision3,
    project_seal: ContentSeal,
    kind: ManagedEntityKind,
    entity_id: EntityId,
    entity_revision: u64,
    module_id: EntityId,
    module_revision: u64,
    persisted_module: Revision3ScriptModule,
}

struct DerivedModule {
    generated: Revision3ScriptModule,
}

enum GuardedDerivation {
    Ready {
        guard: InstallMutationGuard,
        module: DerivedModule,
    },
    Failed {
        guard: InstallMutationGuard,
        failure: Failure,
    },
}

struct FreshGameInputs {
    catalog: StoryCatalogFile,
    shipping: Vec<u8>,
    binds: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClosingRevalidation {
    Exact,
    StoreDrift,
    GameDrift,
    StoreAndGameDrift,
    InspectionFailed,
}

impl ClosingRevalidation {
    fn is_exact(self) -> bool {
        self == Self::Exact
    }
}

pub(super) fn check_revision3_quest_compiler_v1_raw(input: &str) -> Value {
    check_revision3_quest_compiler_v1_inner(input).unwrap_or_else(Failure::response)
}

pub(super) fn check_revision3_npc_compiler_v1_raw(input: &str) -> Value {
    check_revision3_npc_compiler_v1_inner(input).unwrap_or_else(Failure::response)
}

pub(super) fn check_revision3_quest_compiler_v2_raw(input: &str) -> Value {
    check_revision3_quest_compiler_v2_inner(input).unwrap_or_else(Failure::response)
}

pub(super) fn check_revision3_npc_compiler_v2_raw(input: &str) -> Value {
    check_revision3_npc_compiler_v2_inner(input).unwrap_or_else(Failure::response)
}

fn check_revision3_quest_compiler_v2_inner(input: &str) -> Result<Value, Failure> {
    let payload: QuestWirePayloadV2 = parse_exact_wire(input, QUEST_COMMAND_V2)?;
    let requested = payload.compiler_backend;
    let v1 = quest_v1_wire(&payload);
    if requested == CompilerBackendWireV2::Game {
        let (response, game_attempted) = check_revision3_quest_compiler_v1_with_attempt(&v1)?;
        return Ok(attach_managed_backend_evidence(
            response,
            requested,
            game_attempted,
        ));
    }
    let resolution =
        match resolve_product_standalone_compiler_for_game_v1(Path::new(&payload.game_root)) {
            Ok(resolution) => resolution,
            Err(message) if requested == CompilerBackendWireV2::Standalone => {
                let selection = open_quest_selection_v2(&payload)?;
                return strict_standalone_preflight_response(
                    selection,
                    requested,
                    "AUTHORING_REVISION3_STANDALONE_PACKAGE_LOCATION",
                    &message,
                    None,
                );
            }
            Err(message) => {
                let (response, game_attempted) =
                    check_revision3_quest_compiler_v1_with_attempt(&v1)?;
                return Ok(attach_managed_backend_evidence_with_fallback(
                    response,
                    requested,
                    game_attempted,
                    Some(json!({
                        "failed_backend": CompilerBackendNameV1::Standalone.as_str(),
                        "failure_kind": "unavailable",
                        "detail": message,
                    })),
                ));
            }
        };
    match resolution {
        ResolvedProductStandaloneCompilerV1::BundleAbsent => {
            if requested == CompilerBackendWireV2::Standalone {
                let selection = open_quest_selection_v2(&payload)?;
                strict_standalone_preflight_response(
                    selection,
                    requested,
                    "AUTHORING_REVISION3_STANDALONE_BUNDLE_ABSENT",
                    BUNDLE_ABSENT_DETAIL,
                    None,
                )
            } else {
                let (response, game_attempted) =
                    check_revision3_quest_compiler_v1_with_attempt(&v1)?;
                Ok(attach_managed_backend_evidence_with_fallback(
                    response,
                    requested,
                    game_attempted,
                    Some(bundle_absent_fallback_reason()),
                ))
            }
        }
        ResolvedProductStandaloneCompilerV1::Unavailable(reason) => {
            if requested == CompilerBackendWireV2::Standalone {
                let selection = open_quest_selection_v2(&payload)?;
                strict_standalone_preflight_response(
                    selection,
                    requested,
                    "AUTHORING_REVISION3_STANDALONE_PACKAGE_UNAVAILABLE",
                    &format!("{:?}: {}", reason.kind(), reason.detail()),
                    None,
                )
            } else {
                let fallback = package_unavailable_fallback_reason(&reason);
                let (response, game_attempted) =
                    check_revision3_quest_compiler_v1_with_attempt(&v1)?;
                Ok(attach_managed_backend_evidence_with_fallback(
                    response,
                    requested,
                    game_attempted,
                    Some(fallback),
                ))
            }
        }
        ResolvedProductStandaloneCompilerV1::Available(package) => {
            let selection = open_quest_selection_v2(&payload)?;
            run_product_managed_check(
                selection,
                Path::new(&payload.game_root),
                requested,
                package,
                || {
                    derive_quest_module(
                        &payload.expected_head_json,
                        &payload.game_root,
                        &payload.quest_id,
                        &payload.root,
                    )
                },
            )
        }
    }
}

fn check_revision3_npc_compiler_v2_inner(input: &str) -> Result<Value, Failure> {
    let payload: NpcWirePayloadV2 = parse_exact_wire(input, NPC_COMMAND_V2)?;
    let requested = payload.compiler_backend;
    let v1 = npc_v1_wire(&payload);
    if requested == CompilerBackendWireV2::Game {
        let (response, game_attempted) = check_revision3_npc_compiler_v1_with_attempt(&v1)?;
        return Ok(attach_managed_backend_evidence(
            response,
            requested,
            game_attempted,
        ));
    }
    let resolution =
        match resolve_product_standalone_compiler_for_game_v1(Path::new(&payload.game_root)) {
            Ok(resolution) => resolution,
            Err(message) if requested == CompilerBackendWireV2::Standalone => {
                let selection = open_npc_selection_v2(&payload)?;
                return strict_standalone_preflight_response(
                    selection,
                    requested,
                    "AUTHORING_REVISION3_STANDALONE_PACKAGE_LOCATION",
                    &message,
                    None,
                );
            }
            Err(message) => {
                let (response, game_attempted) = check_revision3_npc_compiler_v1_with_attempt(&v1)?;
                return Ok(attach_managed_backend_evidence_with_fallback(
                    response,
                    requested,
                    game_attempted,
                    Some(json!({
                        "failed_backend": CompilerBackendNameV1::Standalone.as_str(),
                        "failure_kind": "unavailable",
                        "detail": message,
                    })),
                ));
            }
        };
    match resolution {
        ResolvedProductStandaloneCompilerV1::BundleAbsent => {
            if requested == CompilerBackendWireV2::Standalone {
                let selection = open_npc_selection_v2(&payload)?;
                strict_standalone_preflight_response(
                    selection,
                    requested,
                    "AUTHORING_REVISION3_STANDALONE_BUNDLE_ABSENT",
                    BUNDLE_ABSENT_DETAIL,
                    None,
                )
            } else {
                let (response, game_attempted) = check_revision3_npc_compiler_v1_with_attempt(&v1)?;
                Ok(attach_managed_backend_evidence_with_fallback(
                    response,
                    requested,
                    game_attempted,
                    Some(bundle_absent_fallback_reason()),
                ))
            }
        }
        ResolvedProductStandaloneCompilerV1::Unavailable(reason) => {
            if requested == CompilerBackendWireV2::Standalone {
                let selection = open_npc_selection_v2(&payload)?;
                strict_standalone_preflight_response(
                    selection,
                    requested,
                    "AUTHORING_REVISION3_STANDALONE_PACKAGE_UNAVAILABLE",
                    &format!("{:?}: {}", reason.kind(), reason.detail()),
                    None,
                )
            } else {
                let fallback = package_unavailable_fallback_reason(&reason);
                let (response, game_attempted) = check_revision3_npc_compiler_v1_with_attempt(&v1)?;
                Ok(attach_managed_backend_evidence_with_fallback(
                    response,
                    requested,
                    game_attempted,
                    Some(fallback),
                ))
            }
        }
        ResolvedProductStandaloneCompilerV1::Available(package) => {
            let selection = open_npc_selection_v2(&payload)?;
            run_product_managed_check(
                selection,
                Path::new(&payload.game_root),
                requested,
                package,
                || derive_npc_module(&payload.expected_head_json, &payload.npc_id, &payload.root),
            )
        }
    }
}

fn strict_standalone_preflight_response(
    selection: InitialSelection,
    requested: CompilerBackendWireV2,
    code: &'static str,
    detail: &str,
    package: Option<
        &gore_as::standalone_package_resolver::ProductStandaloneCompilerPackageIdentityV1,
    >,
) -> Result<Value, Failure> {
    let derived = DerivedModule {
        generated: selection.persisted_module.clone(),
    };
    let mut compiler = preflight_compiler_evidence(code, detail, false);
    compiler["compiler_backend"] =
        backend_evidence_with_package(requested, None, false, false, package, None);
    managed_response(&selection, &derived, compiler, false)
}

fn attach_managed_backend_evidence(
    response: Value,
    requested: CompilerBackendWireV2,
    game_attempted: bool,
) -> Value {
    let fallback = (requested == CompilerBackendWireV2::StandaloneThenGame)
        .then(bundle_absent_fallback_reason);
    attach_managed_backend_evidence_with_fallback(response, requested, game_attempted, fallback)
}

fn attach_managed_backend_evidence_with_fallback(
    mut response: Value,
    requested: CompilerBackendWireV2,
    game_attempted: bool,
    fallback: Option<Value>,
) -> Value {
    let (result_backend, fallback) = match requested {
        CompilerBackendWireV2::Game => {
            (game_attempted.then_some(CompilerBackendNameV1::Game), None)
        }
        CompilerBackendWireV2::StandaloneThenGame => (
            game_attempted.then_some(CompilerBackendNameV1::Game),
            fallback,
        ),
        CompilerBackendWireV2::Standalone => (None, None),
    };
    if response.get("compiler").is_some() {
        response["compiler"]["compiler_backend"] =
            backend_evidence(requested, result_backend, false, game_attempted, fallback);
    }
    response
}

fn quest_v1_wire(payload: &QuestWirePayloadV2) -> String {
    json!({
        "command": QUEST_COMMAND,
        "payload": {
            "expected_head_json": payload.expected_head_json,
            "game_root": payload.game_root,
            "quest_id": payload.quest_id,
            "root": payload.root,
        }
    })
    .to_string()
}

fn npc_v1_wire(payload: &NpcWirePayloadV2) -> String {
    json!({
        "command": NPC_COMMAND,
        "payload": {
            "expected_head_json": payload.expected_head_json,
            "game_root": payload.game_root,
            "npc_id": payload.npc_id,
            "root": payload.root,
        }
    })
    .to_string()
}

fn open_quest_selection_v2(payload: &QuestWirePayloadV2) -> Result<InitialSelection, Failure> {
    let expected_head = parse_canonical_head(&payload.expected_head_json)?;
    let entity_id = parse_entity_id(&payload.quest_id)?;
    validate_paths(&payload.root, &payload.game_root)?;
    open_initial_selection(
        &payload.root,
        expected_head,
        payload.expected_head_json.clone(),
        ManagedEntityKind::Quest,
        entity_id,
    )
}

fn open_npc_selection_v2(payload: &NpcWirePayloadV2) -> Result<InitialSelection, Failure> {
    let expected_head = parse_canonical_head(&payload.expected_head_json)?;
    let entity_id = parse_entity_id(&payload.npc_id)?;
    validate_paths(&payload.root, &payload.game_root)?;
    open_initial_selection(
        &payload.root,
        expected_head,
        payload.expected_head_json.clone(),
        ManagedEntityKind::Npc,
        entity_id,
    )
}

fn derive_quest_module(
    expected_head_json: &str,
    game_root: &str,
    quest_id: &str,
    root: &str,
) -> Result<DerivedModule, Failure> {
    let request = json!({
        "command": crate::authoring_story_quest_inspection_revision3::COMMAND,
        "payload": {
            "expected_head_json": expected_head_json,
            "game_root": game_root,
            "quest_id": quest_id,
            "root": root,
        }
    })
    .to_string();
    let response =
        crate::authoring_story_quest_inspection_revision3::inspect_revision3_quest_source_v1_raw(
            &request,
        );
    let plan_json = inspection_plan_json(response, "Quest")?;
    let plan = Revision3QuestSourceInspectionPlanV3::from_json(&plan_json).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_COMPILER_INVARIANT",
            "native Quest inspection returned an invalid sealed plan",
        )
    })?;
    Ok(DerivedModule {
        generated: plan.module.generated,
    })
}

fn derive_npc_module(
    expected_head_json: &str,
    npc_id: &str,
    root: &str,
) -> Result<DerivedModule, Failure> {
    let request = json!({
        "command": crate::authoring_story_npc_inspection_revision3::COMMAND,
        "payload": {
            "expected_head_json": expected_head_json,
            "npc_id": npc_id,
            "root": root,
        }
    })
    .to_string();
    let response =
        crate::authoring_story_npc_inspection_revision3::inspect_revision3_npc_source_v1_raw(
            &request,
        );
    let plan_json = inspection_plan_json(response, "NPC")?;
    let plan = Revision3NpcSourceInspectionPlanV1::from_json(&plan_json).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_COMPILER_INVARIANT",
            "native NPC inspection returned an invalid sealed plan",
        )
    })?;
    Ok(DerivedModule {
        generated: plan.module().generated().clone(),
    })
}

fn run_product_managed_check<D>(
    selection: InitialSelection,
    game_root: &Path,
    requested: CompilerBackendWireV2,
    package: gore_as::standalone_package_resolver::AvailableProductStandaloneCompilerPackageV1,
    derive: D,
) -> Result<Value, Failure>
where
    D: FnOnce() -> Result<DerivedModule, Failure>,
{
    debug_assert!(requested != CompilerBackendWireV2::Game);
    let initial_derived = DerivedModule {
        generated: selection.persisted_module.clone(),
    };
    let mut guard = if requested == CompilerBackendWireV2::StandaloneThenGame {
        match acquire_compile_install_mutation(game_root) {
            Ok(guard) => Some(guard),
            Err(message) => {
                let mut compiler =
                    compiler_evidence(install_guard_failure(game_root, message), true);
                compiler["compiler_backend"] = backend_evidence_with_package(
                    requested,
                    None,
                    false,
                    false,
                    Some(package.identity()),
                    None,
                );
                return managed_response(&selection, &initial_derived, compiler, false);
            }
        }
    } else {
        None
    };

    let derived = match derive() {
        Ok(module) => module,
        Err(failure) => {
            return product_preflight_failure(
                selection,
                guard.take(),
                &initial_derived,
                requested,
                package.identity(),
                failure,
            );
        }
    };
    if let Err(failure) = validate_derived_module(&selection, &derived) {
        return product_preflight_failure(
            selection,
            guard.take(),
            &initial_derived,
            requested,
            package.identity(),
            failure,
        );
    }

    let (catalog, shipping, binds) = match build_fresh_game_inputs(game_root) {
        Ok(inputs) => inputs,
        Err(failure) => {
            return product_preflight_failure(
                selection,
                guard.take(),
                &derived,
                requested,
                package.identity(),
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
        return product_preflight_failure(
            selection,
            guard.take(),
            &derived,
            requested,
            package.identity(),
            failure,
        );
    }
    if inputs.shipping.as_slice() != package.target_inputs().shipping_cache()
        || inputs.binds.as_slice() != package.target_inputs().binds_cache()
    {
        return product_preflight_failure(
            selection,
            guard.take(),
            &derived,
            requested,
            package.identity(),
            Failure::new(
                "AUTHORING_REVISION3_COMPILER_GAME_MISMATCH",
                "managed inspection inputs do not match the authenticated standalone target",
            ),
        );
    }

    let private_workspace = match tempfile::Builder::new()
        .prefix("gore-managed-compiler-")
        .tempdir()
    {
        Ok(workspace) => workspace,
        Err(_) => {
            return product_preflight_failure(
                selection,
                guard.take(),
                &derived,
                requested,
                package.identity(),
                Failure::new(
                    "AUTHORING_REVISION3_COMPILER_STAGING_UNAVAILABLE",
                    "native-private compiler workspace could not be allocated",
                ),
            );
        }
    };
    let staging = match OwnedCompileStaging::create(private_workspace.path(), game_root) {
        Ok(staging) => staging,
        Err(_) => {
            return product_preflight_failure(
                selection,
                guard.take(),
                &derived,
                requested,
                package.identity(),
                Failure::new(
                    "AUTHORING_REVISION3_COMPILER_STAGING_UNAVAILABLE",
                    "native-private compiler staging could not be created",
                ),
            );
        }
    };
    if staging.verify_owned().is_err() {
        return product_preflight_failure(
            selection,
            guard.take(),
            &derived,
            requested,
            package.identity(),
            Failure::new(
                "AUTHORING_REVISION3_COMPILER_STAGING_CHANGED",
                "native-private compiler staging ownership could not be verified",
            ),
        );
    }

    let mut runner_unavailable = None;
    let mut runner = match package.sidecar_runner(staging.path().to_path_buf()) {
        Ok(runner) => Some(runner),
        Err(failure) if requested == CompilerBackendWireV2::Standalone => {
            let mut detail = Value::String(failure.to_string());
            redact_private_paths(
                &mut detail,
                &[
                    private_workspace.path(),
                    staging.path(),
                    package.sidecar_path(),
                    package.profile_manifest_path(),
                    package.profile_root(),
                ],
            );
            return product_preflight_failure(
                selection,
                None,
                &derived,
                requested,
                package.identity(),
                Failure::new(
                    "AUTHORING_REVISION3_STANDALONE_RUNNER_UNAVAILABLE",
                    detail
                        .as_str()
                        .unwrap_or("standalone runner initialization failed"),
                ),
            );
        }
        Err(failure) => {
            runner_unavailable = Some(json!({
                "failed_backend": CompilerBackendNameV1::Standalone.as_str(),
                "failure_kind": failure.kind().as_str(),
                "detail": failure.detail(),
            }));
            None
        }
    };
    let (authority, target) = package.into_execution_parts();
    let opts = CompileOpts {
        game_dir: game_root.to_path_buf(),
        op: "add".to_owned(),
        module_name: derived.generated.module_namespace.clone(),
        rel_path: derived.generated.module_relative_path.clone(),
        as_path: staging.path().join(".gore-managed-source.as"),
        source_override: Some(derived.generated.source.as_bytes().to_vec()),
        work_dir: staging.path().to_path_buf(),
        allow_new_symbols: true,
        base_override: Some(target.shipping_cache().to_vec()),
        binds_override: Some(target.binds_cache().to_vec()),
    };
    let diagnostics = DiagnosticsOptions {
        disabled: false,
        hook_dll: None,
        inject_delay: Duration::from_secs(2),
    };
    let mut strict_target = Some(target);
    let report = if requested == CompilerBackendWireV2::Standalone {
        compile_module_with_backend_v1(
            &opts,
            &diagnostics,
            CompilerBackendModeV1::Standalone,
            runner.as_mut().map(|runner| runner as _),
        )
    } else {
        compile_module_with_backend_v1_with_guard_and_target(
            &opts,
            &diagnostics,
            CompilerBackendModeV1::StandaloneThenGame,
            runner.as_mut().map(|runner| runner as _),
            guard
                .take()
                .expect("standalone-then-game acquired its guard above"),
            strict_target
                .take()
                .expect("the authenticated target is transferred once"),
        )
    };

    report.finish_while_target_pinned(|mut report| {
        finish_while_target_pinned(strict_target, || {
            let result_backend = report.backend_name();
            let standalone_attempted = report.standalone_attempted();
            let game_attempted = report.game_attempted();
            let mut fallback = runner_unavailable.or_else(|| {
                report.fallback_reason().map(|reason| {
                    json!({
                        "failed_backend": reason.failed_backend().as_str(),
                        "failure_kind": reason.failure_kind().as_str(),
                        "detail": reason.detail(),
                    })
                })
            });
            if let Some(fallback) = fallback.as_mut() {
                redact_private_paths(
                    fallback,
                    &[
                        private_workspace.path(),
                        staging.path(),
                        authority.sidecar_path(),
                        authority.profile_manifest_path(),
                        authority.profile_root(),
                    ],
                );
            }
            let standalone_selected = result_backend == Some(CompilerBackendNameV1::Standalone);
            let (output_discarded, output_rejection) = discard_managed_output(
                &staging,
                &mut report.outcome,
                &derived.generated.module_namespace,
            );
            let script = report_response_with_policy(report, output_rejection, standalone_selected);
            let mut compiler = compiler_evidence_with_private_paths(
                script,
                output_discarded,
                &[private_workspace.path(), staging.path()],
            );
            compiler["compiler_backend"] = backend_evidence_with_package(
                requested,
                result_backend,
                standalone_attempted,
                game_attempted,
                Some(authority.identity()),
                fallback,
            );
            let closing = close_revalidation(&selection, game_root, &inputs);
            managed_response(&selection, &derived, compiler, closing.is_exact())
        })
    })
}

fn finish_while_target_pinned<T, R>(target: Option<T>, finish: impl FnOnce() -> R) -> R {
    let response = finish();
    drop(target);
    response
}

fn product_preflight_failure(
    selection: InitialSelection,
    guard: Option<InstallMutationGuard>,
    derived: &DerivedModule,
    requested: CompilerBackendWireV2,
    package: &gore_as::standalone_package_resolver::ProductStandaloneCompilerPackageIdentityV1,
    failure: Failure,
) -> Result<Value, Failure> {
    let recovery = failure.code.contains("RECOVERY_REQUIRED");
    let mut response = match guard {
        Some(guard) if recovery => release_existing_recovery(selection, guard, derived, failure)?,
        Some(guard) => release_after_preflight(selection, guard, derived, failure)?,
        None => managed_response(
            &selection,
            derived,
            preflight_compiler_evidence(failure.code, failure.message, recovery),
            false,
        )?,
    };
    if response.get("compiler").is_some() {
        response["compiler"]["compiler_backend"] =
            backend_evidence_with_package(requested, None, false, false, Some(package), None);
    }
    Ok(response)
}

fn check_revision3_quest_compiler_v1_inner(input: &str) -> Result<Value, Failure> {
    check_revision3_quest_compiler_v1_with_attempt(input).map(|(response, _)| response)
}

fn check_revision3_quest_compiler_v1_with_attempt(input: &str) -> Result<(Value, bool), Failure> {
    let mut game_attempted = false;
    let response = check_revision3_quest_compiler_v1_recording_attempt(input, &mut game_attempted)?;
    Ok((response, game_attempted))
}

fn check_revision3_quest_compiler_v1_recording_attempt(
    input: &str,
    game_attempted: &mut bool,
) -> Result<Value, Failure> {
    let payload: QuestWirePayload = parse_exact_wire(input, QUEST_COMMAND)?;
    let expected_head = parse_canonical_head(&payload.expected_head_json)?;
    let entity_id = parse_entity_id(&payload.quest_id)?;
    validate_paths(&payload.root, &payload.game_root)?;
    let selection = open_initial_selection(
        &payload.root,
        expected_head,
        payload.expected_head_json.clone(),
        ManagedEntityKind::Quest,
        entity_id,
    )?;
    run_managed_check(
        selection,
        Path::new(&payload.game_root),
        game_attempted,
        |guard| {
            let request = json!({
                "command": crate::authoring_story_quest_inspection_revision3::COMMAND,
                "payload": {
                    "expected_head_json": payload.expected_head_json,
                    "game_root": payload.game_root,
                    "quest_id": payload.quest_id,
                    "root": payload.root,
                }
            })
            .to_string();
            let response = crate::authoring_story_quest_inspection_revision3::inspect_revision3_quest_source_v1_raw(&request);
            let plan_json = match inspection_plan_json(response, "Quest") {
                Ok(plan_json) => plan_json,
                Err(failure) => return GuardedDerivation::Failed { guard, failure },
            };
            let plan = match Revision3QuestSourceInspectionPlanV3::from_json(&plan_json) {
                Ok(plan) => plan,
                Err(_) => {
                    return GuardedDerivation::Failed {
                        guard,
                        failure: Failure::new(
                            "AUTHORING_REVISION3_COMPILER_INVARIANT",
                            "native Quest inspection returned an invalid sealed plan",
                        ),
                    };
                }
            };
            GuardedDerivation::Ready {
                guard,
                module: DerivedModule {
                    generated: plan.module.generated,
                },
            }
        },
    )
}

fn check_revision3_npc_compiler_v1_inner(input: &str) -> Result<Value, Failure> {
    check_revision3_npc_compiler_v1_with_attempt(input).map(|(response, _)| response)
}

fn check_revision3_npc_compiler_v1_with_attempt(input: &str) -> Result<(Value, bool), Failure> {
    let mut game_attempted = false;
    let response = check_revision3_npc_compiler_v1_recording_attempt(input, &mut game_attempted)?;
    Ok((response, game_attempted))
}

fn check_revision3_npc_compiler_v1_recording_attempt(
    input: &str,
    game_attempted: &mut bool,
) -> Result<Value, Failure> {
    let payload: NpcWirePayload = parse_exact_wire(input, NPC_COMMAND)?;
    let expected_head = parse_canonical_head(&payload.expected_head_json)?;
    let entity_id = parse_entity_id(&payload.npc_id)?;
    validate_paths(&payload.root, &payload.game_root)?;
    let selection = open_initial_selection(
        &payload.root,
        expected_head,
        payload.expected_head_json.clone(),
        ManagedEntityKind::Npc,
        entity_id,
    )?;
    run_managed_check(
        selection,
        Path::new(&payload.game_root),
        game_attempted,
        |guard| {
            let request = json!({
                "command": crate::authoring_story_npc_inspection_revision3::COMMAND,
                "payload": {
                    "expected_head_json": payload.expected_head_json,
                    "npc_id": payload.npc_id,
                    "root": payload.root,
                }
            })
            .to_string();
            let response =
            crate::authoring_story_npc_inspection_revision3::inspect_revision3_npc_source_v1_raw(
                &request,
            );
            let plan_json = match inspection_plan_json(response, "NPC") {
                Ok(plan_json) => plan_json,
                Err(failure) => return GuardedDerivation::Failed { guard, failure },
            };
            let plan = match Revision3NpcSourceInspectionPlanV1::from_json(&plan_json) {
                Ok(plan) => plan,
                Err(_) => {
                    return GuardedDerivation::Failed {
                        guard,
                        failure: Failure::new(
                            "AUTHORING_REVISION3_COMPILER_INVARIANT",
                            "native NPC inspection returned an invalid sealed plan",
                        ),
                    };
                }
            };
            GuardedDerivation::Ready {
                guard,
                module: DerivedModule {
                    generated: plan.module().generated().clone(),
                },
            }
        },
    )
}

fn run_managed_check<D>(
    selection: InitialSelection,
    game_root: &Path,
    game_attempted: &mut bool,
    derive: D,
) -> Result<Value, Failure>
where
    D: FnOnce(InstallMutationGuard) -> GuardedDerivation,
{
    // This exact persisted module is fully Store/head bound before any install ownership or live
    // attempt. The inspection primitive below must independently regenerate the same bytes before
    // they can be compiled.
    let initial_derived = DerivedModule {
        generated: selection.persisted_module.clone(),
    };
    let guard = match acquire_compile_install_mutation(game_root) {
        Ok(guard) => guard,
        Err(message) => {
            let compiler = compiler_evidence(install_guard_failure(game_root, message), true);
            return managed_response(&selection, &initial_derived, compiler, false);
        }
    };

    let (guard, derived) = match derive(guard) {
        GuardedDerivation::Ready { guard, module } => (guard, module),
        GuardedDerivation::Failed { guard, failure }
            if failure.code.contains("RECOVERY_REQUIRED") =>
        {
            return release_existing_recovery(selection, guard, &initial_derived, failure);
        }
        GuardedDerivation::Failed { guard, failure } => {
            return release_after_preflight(selection, guard, &initial_derived, failure);
        }
    };
    if let Err(failure) = validate_derived_module(&selection, &derived) {
        return release_after_preflight(selection, guard, &initial_derived, failure);
    }

    let (catalog, shipping, binds) = match build_fresh_game_inputs(game_root) {
        Ok(inputs) => inputs,
        Err(failure) => {
            let managed = Failure::new(map_game_input_code(failure.code), failure.message);
            if failure.code.contains("RECOVERY_REQUIRED") {
                return release_existing_recovery(selection, guard, &derived, managed);
            }
            return release_after_preflight(selection, guard, &derived, managed);
        }
    };
    let inputs = FreshGameInputs {
        catalog,
        shipping,
        binds,
    };
    if let Err(failure) = validate_game_target(&selection.project, &inputs.catalog) {
        return release_after_preflight(selection, guard, &derived, failure);
    }

    // The compiler tree and mini-cache never live below a wire-selected path. Keeping the entire
    // tree in a native-private, unreported workspace prevents a caller from replacing emitted
    // source/base files between sealing and the game process reading them.
    let private_workspace = match tempfile::Builder::new()
        .prefix("gore-managed-compiler-")
        .tempdir()
    {
        Ok(workspace) => workspace,
        Err(_) => {
            return release_after_preflight(
                selection,
                guard,
                &derived,
                Failure::new(
                    "AUTHORING_REVISION3_COMPILER_STAGING_UNAVAILABLE",
                    "native-private compiler workspace could not be allocated",
                ),
            );
        }
    };
    let staging = match OwnedCompileStaging::create(private_workspace.path(), game_root) {
        Ok(staging) => staging,
        Err(_) => {
            return release_after_preflight(
                selection,
                guard,
                &derived,
                Failure::new(
                    "AUTHORING_REVISION3_COMPILER_STAGING_UNAVAILABLE",
                    "native-private compiler staging could not be created",
                ),
            );
        }
    };
    if staging.verify_owned().is_err() {
        return release_after_preflight(
            selection,
            guard,
            &derived,
            Failure::new(
                "AUTHORING_REVISION3_COMPILER_STAGING_CHANGED",
                "native-private compiler staging ownership could not be verified",
            ),
        );
    }

    let opts = CompileOpts {
        game_dir: game_root.to_path_buf(),
        op: "add".to_owned(),
        module_name: derived.generated.module_namespace.clone(),
        rel_path: derived.generated.module_relative_path.clone(),
        // This path is deliberately never opened: the exact native-derived source bytes below
        // are the compiler input, so a caller-controlled workspace cannot race the source seal.
        as_path: staging.path().join(".gore-managed-source.as"),
        source_override: Some(derived.generated.source.as_bytes().to_vec()),
        work_dir: staging.path().to_path_buf(),
        allow_new_symbols: true,
        base_override: Some(inputs.shipping.clone()),
        binds_override: Some(inputs.binds.clone()),
    };
    let mut report = compile_module_with_diagnostics_report_with_guard(
        &opts,
        &DiagnosticsOptions {
            disabled: false,
            hook_dll: None,
            inject_delay: Duration::from_secs(2),
        },
        guard,
    );
    *game_attempted = report.game_attempted();
    let (output_discarded, output_rejection) = discard_managed_output(
        &staging,
        &mut report.outcome,
        &derived.generated.module_namespace,
    );
    let compiler = compiler_evidence_with_private_paths(
        report_response(report, output_rejection),
        output_discarded,
        &[private_workspace.path(), staging.path()],
    );
    let closing = close_revalidation(&selection, game_root, &inputs);
    managed_response(&selection, &derived, compiler, closing.is_exact())
}

fn discard_managed_output(
    staging: &OwnedCompileStaging,
    outcome: &mut CompileModuleReportOutcome,
    expected_module: &str,
) -> (bool, Option<String>) {
    match outcome {
        CompileModuleReportOutcome::Compiled(output) => {
            let mut rejections = Vec::new();
            if output.module_name != expected_module {
                rejections.push(format!(
                    "compiler returned module {:?}, expected the exact managed namespace {:?}",
                    output.module_name, expected_module
                ));
            }
            let output_discarded = match discard_owned_compiled_mini(staging, output) {
                Ok(()) => true,
                Err(message) => {
                    rejections.push(message);
                    false
                }
            };
            let rejection = (!rejections.is_empty()).then(|| rejections.join("; "));
            (output_discarded, rejection)
        }
        CompileModuleReportOutcome::Failed(error) => {
            match discard_owned_failed_compiled_mini(staging, error) {
                Ok(()) => (true, None),
                Err(message) => (false, Some(message)),
            }
        }
    }
}

fn release_after_preflight(
    selection: InitialSelection,
    mut guard: InstallMutationGuard,
    derived: &DerivedModule,
    failure: Failure,
) -> Result<Value, Failure> {
    match guard.release() {
        Ok(()) => managed_response(
            &selection,
            derived,
            preflight_compiler_evidence(failure.code, failure.message, false),
            false,
        ),
        Err(release) => {
            guard.preserve_for_manual_recovery();
            let compiler = preflight_compiler_evidence(
                "COMPILE_INSTALL_GUARD_RELEASE_FAILED",
                format!(
                    "{}; install guard release failed: {release}",
                    failure.message
                ),
                true,
            );
            managed_response(&selection, derived, compiler, false)
        }
    }
}

fn release_existing_recovery(
    selection: InitialSelection,
    mut guard: InstallMutationGuard,
    derived: &DerivedModule,
    failure: Failure,
) -> Result<Value, Failure> {
    let mut message = failure.message;
    let mut code = "COMPILE_INSTALL_RECOVERY_REQUIRED";
    if let Err(release) = guard.release() {
        guard.preserve_for_manual_recovery();
        message = format!("{message}; install guard release failed: {release}");
        code = "COMPILE_INSTALL_GUARD_RELEASE_FAILED";
    }
    let compiler = preflight_compiler_evidence(code, message, true);
    managed_response(&selection, derived, compiler, false)
}

fn open_initial_selection(
    root: &str,
    expected_head: WorkingHead,
    expected_head_json: String,
    kind: ManagedEntityKind,
    entity_id: EntityId,
) -> Result<InitialSelection, Failure> {
    let store = WorkingProjectStore::open_existing(Path::new(root), ffi_store_limits())
        .map_err(map_store_error)?;
    let opened = store
        .open_current_revision3(AssetVerification::Full)
        .map_err(map_store_error)?;
    if opened.head != expected_head {
        return Err(head_conflict());
    }
    let canonical_head = serde_json::to_string(&opened.head).map_err(|_| invariant_failure())?;
    if canonical_head != expected_head_json {
        return Err(Failure::new(
            "AUTHORING_REVISION3_COMPILER_HEAD_INVALID",
            "expected_head_json is not exact canonical JSON",
        ));
    }
    let canonical_project = opened
        .project
        .to_canonical_json()
        .map_err(|_| invariant_failure())?;
    let project_seal = seal_bytes(canonical_project.as_bytes());
    let entity = opened.project.entities.get(&entity_id).ok_or_else(|| {
        Failure::new(
            "AUTHORING_REVISION3_COMPILER_ENTITY_INVALID",
            "the selected managed entity does not exist in the exact revision-3 project",
        )
    })?;
    let module_ref = match (&entity.payload, kind) {
        (Revision3EntityPayload::QuestDraft(draft), ManagedEntityKind::Quest) => {
            &draft.script_module
        }
        (Revision3EntityPayload::NpcDraft(draft), ManagedEntityKind::Npc) => &draft.script_module,
        _ => {
            return Err(Failure::new(
                "AUTHORING_REVISION3_COMPILER_ENTITY_INVALID",
                "the selected id does not have the requested managed entity kind",
            ));
        }
    };
    if module_ref.project_id != opened.project.project_id
        || module_ref.expected_kind != Revision3EntityKind::ScriptModule
    {
        return Err(invariant_failure());
    }
    let module = opened
        .project
        .entities
        .get(&module_ref.id)
        .ok_or_else(invariant_failure)?;
    let Revision3EntityPayload::ScriptModule(persisted_module) = &module.payload else {
        return Err(invariant_failure());
    };
    let entity_revision = entity.revision;
    let module_id = module_ref.id;
    let module_revision = module.revision;
    let persisted_module = persisted_module.clone();
    for value in [
        opened.project.revision,
        entity.revision,
        module.revision,
        project_seal.byte_len,
    ] {
        signed_wire_u64(value)?;
    }
    Ok(InitialSelection {
        store,
        expected_head,
        expected_head_json,
        project: opened.project,
        project_seal,
        kind,
        entity_id,
        entity_revision,
        module_id,
        module_revision,
        persisted_module,
    })
}

fn validate_derived_module(
    selection: &InitialSelection,
    derived: &DerivedModule,
) -> Result<(), Failure> {
    let module = selection
        .project
        .entities
        .get(&selection.module_id)
        .ok_or_else(invariant_failure)?;
    let Revision3EntityPayload::ScriptModule(persisted) = &module.payload else {
        return Err(invariant_failure());
    };
    if &derived.generated != persisted
        || derived.generated.owner.project_id != selection.project.project_id
        || derived.generated.owner.id != selection.entity_id
        || derived.generated.owner.expected_kind != selection.kind.expected_kind()
        || derived.generated.module_namespace.is_empty()
        || derived.generated.module_relative_path.is_empty()
        || derived.generated.source.is_empty()
    {
        return Err(Failure::new(
            "AUTHORING_REVISION3_COMPILER_SOURCE_DRIFT",
            "persisted and natively regenerated managed source are not exact",
        ));
    }
    let expected_path = format!(
        "{}.as",
        derived.generated.module_namespace.replace('.', "/")
    );
    let actual_sha =
        Sha256Digest::from_bytes(Sha256::digest(derived.generated.source.as_bytes()).into());
    if derived.generated.module_relative_path != expected_path
        || derived.generated.source_sha256 != actual_sha
    {
        return Err(invariant_failure());
    }
    Ok(())
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
            "AUTHORING_REVISION3_COMPILER_GAME_MISMATCH",
            "the selected game executable does not match the project's sealed target",
        ));
    }
    Ok(())
}

fn close_revalidation(
    selection: &InitialSelection,
    game_root: &Path,
    inputs: &FreshGameInputs,
) -> ClosingRevalidation {
    let store = revalidate_store(selection);
    let game = match revalidate_game_inputs(&inputs.catalog, game_root, &inputs.shipping) {
        Ok(()) => Some(true),
        Err(failure) if failure.code.ends_with("_INPUT_CHANGED") => Some(false),
        Err(_) => None,
    };
    closing_from_parts(store, game)
}

fn closing_from_parts(store: Option<bool>, game: Option<bool>) -> ClosingRevalidation {
    match (store, game) {
        (Some(true), Some(true)) => ClosingRevalidation::Exact,
        (Some(false), Some(true)) => ClosingRevalidation::StoreDrift,
        (Some(true), Some(false)) => ClosingRevalidation::GameDrift,
        (Some(false), Some(false)) => ClosingRevalidation::StoreAndGameDrift,
        _ => ClosingRevalidation::InspectionFailed,
    }
}

fn revalidate_store(selection: &InitialSelection) -> Option<bool> {
    selection
        .store
        .open_current_revision3(AssetVerification::Full)
        .ok()
        .map(|opened| opened.head == selection.expected_head && opened.project == selection.project)
}

fn inspection_plan_json(response: Value, label: &str) -> Result<String, Failure> {
    if response.get("ok") == Some(&Value::Bool(true)) {
        return response
            .get("plan_json")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(invariant_failure);
    }
    let code = response
        .pointer("/error/code")
        .and_then(Value::as_str)
        .unwrap_or("AUTHORING_REVISION3_COMPILER_INSPECTION_FAILED");
    let message = response
        .pointer("/error/message")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("native {label} inspection failed"));
    let mapped = if code.contains("RECOVERY_REQUIRED") {
        "AUTHORING_REVISION3_COMPILER_RECOVERY_REQUIRED"
    } else if code.contains("HEAD_CONFLICT") {
        "AUTHORING_REVISION3_COMPILER_HEAD_CONFLICT"
    } else if code.contains("INPUT_LIMIT") {
        "AUTHORING_REVISION3_COMPILER_INPUT_LIMIT"
    } else {
        "AUTHORING_REVISION3_COMPILER_INSPECTION_FAILED"
    };
    Err(Failure::new(mapped, message))
}

fn compiler_evidence(script: Value, output_discarded: bool) -> Value {
    compiler_evidence_with_private_paths(script, output_discarded, &[])
}

fn compiler_evidence_with_private_paths(
    script: Value,
    output_discarded: bool,
    private_paths: &[&Path],
) -> Value {
    let compiled =
        script.get("outcome").and_then(Value::as_str) == Some("compiled") && output_discarded;
    let mut compile_error = script.get("compile_error").cloned().unwrap_or(Value::Null);
    let mut compiler_diagnostics = script
        .get("compiler_diagnostics")
        .cloned()
        .unwrap_or(Value::Null);
    redact_private_paths(&mut compile_error, private_paths);
    redact_private_paths(&mut compiler_diagnostics, private_paths);
    json!({
        "outcome": if compiled { "compiled_evidence_only" } else { "failed" },
        "compile_error": compile_error,
        "compiler_diagnostics": compiler_diagnostics,
        "install_restore": script.get("install_restore").and_then(Value::as_str).unwrap_or("not_started"),
        "recovery_required": script.get("recovery_required").and_then(Value::as_bool).unwrap_or(false),
        "output_discarded": output_discarded,
    })
}

pub(super) fn redact_private_paths(value: &mut Value, private_paths: &[&Path]) {
    let variants = private_path_variants(private_paths);
    redact_private_path_variants(value, &variants);
}

fn private_path_variants(private_paths: &[&Path]) -> Vec<String> {
    let mut variants = Vec::new();
    for path in private_paths {
        push_path_variant(&mut variants, path.to_string_lossy().into_owned());
        if let Ok(canonical) = path.canonicalize() {
            push_path_variant(&mut variants, canonical.to_string_lossy().into_owned());
        }
    }

    for path in variants.clone() {
        push_path_variant(&mut variants, path.replace('\\', "/"));
        push_path_variant(&mut variants, path.replace('/', "\\"));

        if let Some(path) = path.strip_prefix(r"\\?\UNC\") {
            push_path_variant(&mut variants, format!(r"\\{path}"));
        } else if let Some(path) = path.strip_prefix(r"\\?\") {
            push_path_variant(&mut variants, path.to_owned());
        } else if path.as_bytes().get(1) == Some(&b':') {
            push_path_variant(&mut variants, format!(r"\\?\{path}"));
        }
    }

    variants.sort_unstable_by_key(|path| std::cmp::Reverse(path.len()));
    variants.dedup();
    variants
}

fn push_path_variant(variants: &mut Vec<String>, path: String) {
    if !path.is_empty() && !variants.iter().any(|candidate| candidate == &path) {
        variants.push(path);
    }
}

fn redact_private_path_variants(value: &mut Value, variants: &[String]) {
    match value {
        Value::String(text) => {
            for variant in variants {
                *text = replace_ascii_case_insensitive(text, variant, PRIVATE_PATH_REDACTION);
            }
        }
        Value::Array(values) => {
            for value in values {
                redact_private_path_variants(value, variants);
            }
        }
        Value::Object(fields) => {
            for value in fields.values_mut() {
                redact_private_path_variants(value, variants);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn replace_ascii_case_insensitive(text: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return text.to_owned();
    }

    let mut output = String::with_capacity(text.len());
    let mut remaining = text;
    while let Some(index) = remaining.char_indices().find_map(|(index, _)| {
        let end = index.checked_add(needle.len())?;
        (end <= remaining.len()
            && remaining.is_char_boundary(end)
            && remaining[index..end].eq_ignore_ascii_case(needle))
        .then_some(index)
    }) {
        output.push_str(&remaining[..index]);
        output.push_str(replacement);
        remaining = &remaining[index + needle.len()..];
    }
    output.push_str(remaining);
    output
}

fn preflight_compiler_evidence(
    code: &'static str,
    message: impl Into<String>,
    recovery_required: bool,
) -> Value {
    json!({
        "outcome": "failed",
        "compile_error": {
            "code": code,
            "message": truncate_utf8(message.into(), MAX_ERROR_MESSAGE_BYTES),
        },
        "compiler_diagnostics": Value::Null,
        "install_restore": "not_started",
        "recovery_required": recovery_required,
        "output_discarded": true,
    })
}

fn managed_response(
    selection: &InitialSelection,
    derived: &DerivedModule,
    compiler: Value,
    inputs_exact: bool,
) -> Result<Value, Failure> {
    let recovery = compiler
        .get("recovery_required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let exact_current = inputs_exact && !recovery;
    let module = json!({
        "id": selection.module_id.to_string(),
        "revision": selection.module_revision,
        "namespace": derived.generated.module_namespace,
        "relative_path": derived.generated.module_relative_path,
        "source_sha256": derived.generated.source_sha256.to_string(),
    });
    let response = json!({
        "ok": true,
        "outcome": "compiler_check_only",
        "exact_current": exact_current,
        "head_json": selection.expected_head_json,
        "project": {
            "id": selection.project.project_id.to_string(),
            "revision": selection.project.revision,
            "seal": selection.project_seal,
        },
        "entity": {
            "kind": selection.kind.label(),
            "id": selection.entity_id.to_string(),
            "revision": selection.entity_revision,
        },
        "module": module,
        "compiler": compiler,
        "scope": "compiler_check_only",
        "build_status": "blocked",
        "deploy_status": "not_supported",
        "runtime_qualification": "runtime_unqualified",
        "publication_status": "not_supported",
    });
    enforce_response_budget(response)
}

fn parse_exact_wire<P: DeserializeOwned>(input: &str, command: &str) -> Result<P, Failure> {
    if input.len() > MAX_WIRE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_COMPILER_INPUT_LIMIT",
            "managed compiler-check request exceeds its bounded wire limit",
        ));
    }
    let request: ExactWireRequest<P> =
        serde_json::from_str(input).map_err(|_| invalid_request())?;
    if request.command != command {
        return Err(invalid_request());
    }
    Ok(request.payload)
}

fn parse_canonical_head(input: &str) -> Result<WorkingHead, Failure> {
    if input.is_empty() || input.len() > MAX_HEAD_JSON_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_COMPILER_HEAD_INVALID",
            "expected_head_json is empty or exceeds its bounded transport limit",
        ));
    }
    let head: WorkingHead = serde_json::from_str(input).map_err(|_| {
        Failure::new(
            "AUTHORING_REVISION3_COMPILER_HEAD_INVALID",
            "expected_head_json is not one closed working head",
        )
    })?;
    if serde_json::to_string(&head).ok().as_deref() != Some(input) {
        return Err(Failure::new(
            "AUTHORING_REVISION3_COMPILER_HEAD_INVALID",
            "expected_head_json is not duplicate-free canonical JSON",
        ));
    }
    Ok(head)
}

fn parse_entity_id(input: &str) -> Result<EntityId, Failure> {
    if input.len() != ENTITY_ID_BYTES {
        return Err(invalid_request());
    }
    input.parse().map_err(|_| invalid_request())
}

fn validate_paths(root: &str, game_root: &str) -> Result<(), Failure> {
    for path in [root, game_root] {
        if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\0') {
            return Err(invalid_request());
        }
    }
    Ok(())
}

fn map_game_input_code(code: &str) -> &'static str {
    if code.contains("RECOVERY_REQUIRED") {
        "AUTHORING_REVISION3_COMPILER_RECOVERY_REQUIRED"
    } else if code.contains("INPUT_LIMIT") {
        "AUTHORING_REVISION3_COMPILER_INPUT_LIMIT"
    } else if code.contains("INPUT_CHANGED") {
        "AUTHORING_REVISION3_COMPILER_GAME_DRIFT"
    } else if code.contains("UNSUPPORTED_GENERATION") {
        "AUTHORING_REVISION3_COMPILER_UNSUPPORTED_GENERATION"
    } else {
        "AUTHORING_REVISION3_COMPILER_GAME_INPUT_UNAVAILABLE"
    }
}

fn map_store_error(error: WorkingStoreError) -> Failure {
    let code = match error {
        WorkingStoreError::MissingRoot(_) => "AUTHORING_REVISION3_COMPILER_STORE_ROOT_MISSING",
        WorkingStoreError::MissingHead(_) => "AUTHORING_REVISION3_COMPILER_HEAD_MISSING",
        WorkingStoreError::HeadConflict { .. } => "AUTHORING_REVISION3_COMPILER_HEAD_CONFLICT",
        WorkingStoreError::LimitExceeded { .. } => "AUTHORING_REVISION3_COMPILER_INPUT_LIMIT",
        _ => "AUTHORING_REVISION3_COMPILER_STORE_INVALID",
    };
    Failure::new(
        code,
        "the exact revision-3 working Store could not be opened",
    )
}

fn ffi_store_limits() -> WorkingStoreLimits {
    WorkingStoreLimits {
        max_referenced_entity_bytes: MAX_PROJECT_JSON_BYTES as u64,
        ..WorkingStoreLimits::default()
    }
}

fn signed_wire_u64(value: u64) -> Result<(), Failure> {
    if value > i64::MAX as u64 {
        return Err(Failure::new(
            "AUTHORING_REVISION3_COMPILER_RESPONSE_LIMIT",
            "managed compiler-check evidence contains an integer outside the signed wire range",
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
    let encoded = serde_json::to_vec(&response).map_err(|_| invariant_failure())?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err(Failure::new(
            "AUTHORING_REVISION3_COMPILER_RESPONSE_LIMIT",
            "managed compiler-check response exceeds its bounded transport budget",
        ));
    }
    Ok(response)
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_COMPILER_REQUEST_INVALID",
        "request must contain one exact duplicate-free command and only root, game_root, expected_head_json, and the selected entity id",
    )
}

fn head_conflict() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_COMPILER_HEAD_CONFLICT",
        "the published revision-3 head changed or differs from the caller's exact head",
    )
}

fn invariant_failure() -> Failure {
    Failure::new(
        "AUTHORING_REVISION3_COMPILER_INVARIANT",
        "native managed compiler-check invariants were not satisfied",
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
    use std::path::{Path, PathBuf};

    use gore_as::compile::acquire_compile_install_mutation_with_stated_game_process;
    use gore_authoring::{
        AssetStoreIndex, FormatV2, GameGenerationAnchor, NpcParentClassInput, ProjectId,
        ProjectMeta, Revision3Entity, Revision3NpcDraft, Revision3NpcDraftInput,
        Revision3OriginRef, Revision3TypedRef, SchemaRevisionV3, LOGICAL_NPC_CLONE_GENERATOR_ID,
        LOGICAL_NPC_CLONE_GENERATOR_VERSION,
    };
    use tempfile::TempDir;

    use super::*;

    const NPC_BYTE: u8 = 0x61;
    const MODULE_BYTE: u8 = 0x62;

    fn digest(value: u8) -> Sha256Digest {
        Sha256Digest::from_bytes([value; 32])
    }

    fn content_seal(value: u8, byte_len: u64) -> ContentSeal {
        ContentSeal {
            byte_len,
            sha256: digest(value),
        }
    }

    fn target() -> gore_authoring::GameGenerationAnchor {
        GameGenerationAnchor {
            executable: content_seal(1, 171_698_176),
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
        let project_id = ProjectId::from_bytes([0x60; 16]);
        let npc_id = EntityId::from_bytes([NPC_BYTE; 16]);
        let module_id = EntityId::from_bytes([MODULE_BYTE; 16]);
        let owner = Revision3TypedRef::new(project_id, npc_id, Revision3EntityKind::NpcDraft);
        let draft = Revision3NpcDraft {
            generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
            generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
            input: Revision3NpcDraftInput {
                target: target(),
                module_namespace: "GoreMods.Npcs.ManagedCompilerCheck".to_owned(),
                unique_name: "GORE_MANAGED_COMPILER_CHECK".to_owned(),
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
        let module = draft.regenerate_script_module(owner.clone()).unwrap();
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id,
            revision,
            meta: ProjectMeta {
                name: "Managed compiler check".to_owned(),
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
                        display_name: "Managed compiler NPC".to_owned(),
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
                        display_name: "Managed compiler source".to_owned(),
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

    fn published_store(project: &ProjectRevision3) -> (TempDir, String) {
        let temp = TempDir::new().unwrap();
        let store = WorkingProjectStore::at(temp.path(), ffi_store_limits()).unwrap();
        let prepared = store.prepare_revision3_checkpoint(None, project).unwrap();
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

    fn npc_wire(payload: Value) -> String {
        json!({"command": NPC_COMMAND, "payload": payload}).to_string()
    }

    fn npc_wire_v2(payload: Value) -> String {
        json!({"command": NPC_COMMAND_V2, "payload": payload}).to_string()
    }

    fn valid_npc_shape() -> Value {
        json!({
            "expected_head_json": "{}",
            "game_root": "C:/missing-game",
            "npc_id": EntityId::from_bytes([NPC_BYTE; 16]).to_string(),
            "root": "C:/missing-store",
        })
    }

    #[test]
    fn exact_wire_rejects_all_caller_forged_compiler_authority() {
        let parsed: NpcWirePayload = parse_exact_wire(&npc_wire(valid_npc_shape()), NPC_COMMAND)
            .expect("exact shape parses");
        assert_eq!(parsed.game_root, "C:/missing-game");

        for (field, value) in [
            ("source", json!("forged source")),
            ("module", json!("GoreMods.Forged")),
            ("module_name", json!("GoreMods.Forged")),
            ("rel_path", json!("GoreMods/Forged.as")),
            ("as_path", json!("C:/forged.as")),
            ("work_dir", json!("C:/caller-controlled-work")),
            ("op", json!("edit")),
            ("allow_new_symbols", json!(false)),
            ("build", json!(true)),
            ("deploy", json!(true)),
        ] {
            let mut payload = valid_npc_shape();
            payload[field] = value;
            assert_eq!(
                check_revision3_npc_compiler_v1_raw(&npc_wire(payload))["error"]["code"],
                "AUTHORING_REVISION3_COMPILER_REQUEST_INVALID",
                "accepted forged field {field}"
            );
        }

        let duplicate = format!(
            "{{\"command\":\"{NPC_COMMAND}\",\"payload\":{{\"expected_head_json\":\"{{}}\",\"game_root\":\"C:/g\",\"npc_id\":\"{}\",\"npc_id\":\"{}\",\"root\":\"C:/r\"}}}}",
            EntityId::from_bytes([NPC_BYTE; 16]),
            EntityId::from_bytes([NPC_BYTE; 16]),
        );
        assert_eq!(
            check_revision3_npc_compiler_v1_raw(&duplicate)["error"]["code"],
            "AUTHORING_REVISION3_COMPILER_REQUEST_INVALID"
        );

        let forged_quest = json!({
            "command": QUEST_COMMAND,
            "payload": {
                "expected_head_json": "{}",
                "game_root": "C:/missing-game",
                "quest_id": EntityId::from_bytes([0x71; 16]),
                "root": "C:/missing-store",
                "allow_new_symbols": true,
            }
        })
        .to_string();
        assert_eq!(
            check_revision3_quest_compiler_v1_raw(&forged_quest)["error"]["code"],
            "AUTHORING_REVISION3_COMPILER_REQUEST_INVALID"
        );
    }

    #[test]
    fn v2_npc_strict_standalone_is_store_bound_and_fails_before_game_ownership() {
        let project = npc_project(7);
        let (temp, head_json) = published_store(&project);
        let game = temp.path().join("missing-game");
        let payload = json!({
            "compiler_backend": "standalone",
            "expected_head_json": head_json,
            "game_root": game.display().to_string(),
            "npc_id": EntityId::from_bytes([NPC_BYTE; 16]).to_string(),
            "root": temp.path().display().to_string(),
        });

        let response = crate::dispatch(&npc_wire_v2(payload));

        assert_eq!(response["ok"], true);
        assert_eq!(response["compiler"]["outcome"], "failed");
        assert_eq!(
            response["compiler"]["compile_error"]["code"],
            "AUTHORING_REVISION3_STANDALONE_BUNDLE_ABSENT"
        );
        assert_eq!(
            response["compiler"]["compiler_backend"]["requested_mode"],
            "standalone"
        );
        assert!(response["compiler"]["compiler_backend"]["result_backend"].is_null());
        assert_eq!(
            response["compiler"]["compiler_backend"]["standalone_attempted"],
            false
        );
        assert_eq!(
            response["compiler"]["compiler_backend"]["game_attempted"],
            false
        );
        assert!(response["compiler"]["compiler_backend"]["qualified_package"].is_null());
        assert_eq!(response["exact_current"], false);
        assert!(!game.join(".gore-install-mutation.lock").exists());
    }

    #[test]
    fn v2_npc_wire_accepts_only_backend_policy_not_package_authority() {
        let mut payload = valid_npc_shape();
        payload["compiler_backend"] = json!("standalone");
        payload["compiler_profile_root"] = json!("C:/caller/profile");
        let response = check_revision3_npc_compiler_v2_raw(&npc_wire_v2(payload));
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_COMPILER_REQUEST_INVALID"
        );

        let forged_quest = json!({
            "command": QUEST_COMMAND_V2,
            "payload": {
                "compiler_backend": "standalone",
                "expected_head_json": "{}",
                "game_root": "C:/missing-game",
                "quest_id": EntityId::from_bytes([0x71; 16]),
                "root": "C:/missing-store",
                "sidecar_sha256": "forged",
            }
        })
        .to_string();
        assert_eq!(
            check_revision3_quest_compiler_v2_raw(&forged_quest)["error"]["code"],
            "AUTHORING_REVISION3_COMPILER_REQUEST_INVALID"
        );
    }

    #[test]
    fn v2_managed_fallback_evidence_preserves_bundle_absence() {
        let response = attach_managed_backend_evidence(
            json!({"ok": true, "compiler": {"outcome": "failed"}}),
            CompilerBackendWireV2::StandaloneThenGame,
            false,
        );
        assert_eq!(
            response["compiler"]["compiler_backend"]["requested_mode"],
            "standalone_then_game"
        );
        assert!(response["compiler"]["compiler_backend"]["result_backend"].is_null());
        assert_eq!(
            response["compiler"]["compiler_backend"]["standalone_attempted"],
            false
        );
        assert_eq!(
            response["compiler"]["compiler_backend"]["game_attempted"],
            false
        );
        assert_eq!(
            response["compiler"]["compiler_backend"]["fallback_reason"]["failure_kind"],
            "unavailable"
        );
        assert_eq!(
            response["compiler"]["compiler_backend"]["fallback_reason"]["detail"],
            BUNDLE_ABSENT_DETAIL
        );

        let attempted = attach_managed_backend_evidence(
            json!({
                "ok": true,
                "compiler": {"outcome": "failed", "install_restore": "not_started"}
            }),
            CompilerBackendWireV2::Game,
            true,
        );
        assert_eq!(
            attempted["compiler"]["compiler_backend"]["result_backend"],
            "game"
        );
        assert_eq!(
            attempted["compiler"]["compiler_backend"]["game_attempted"],
            true
        );
    }

    #[test]
    fn initial_exact_selection_is_read_only_and_binds_persisted_module() {
        let project = npc_project(7);
        let (temp, head_json) = published_store(&project);
        let before = snapshot_regular_files(temp.path());
        let head: WorkingHead = serde_json::from_str(&head_json).unwrap();
        let selection = open_initial_selection(
            temp.path().to_str().unwrap(),
            head,
            head_json.clone(),
            ManagedEntityKind::Npc,
            EntityId::from_bytes([NPC_BYTE; 16]),
        )
        .unwrap();
        assert_eq!(selection.expected_head_json, head_json);
        assert_eq!(selection.entity_revision, 2);
        assert_eq!(selection.module_revision, 3);
        assert_eq!(
            selection.persisted_module.module_namespace,
            "GoreMods.Npcs.ManagedCompilerCheck"
        );
        assert_eq!(snapshot_regular_files(temp.path()), before);
        assert_eq!(revalidate_store(&selection), Some(true));
    }

    #[test]
    fn closing_store_revalidation_detects_exact_head_drift() {
        let project = npc_project(7);
        let (temp, head_json) = published_store(&project);
        let head: WorkingHead = serde_json::from_str(&head_json).unwrap();
        let selection = open_initial_selection(
            temp.path().to_str().unwrap(),
            head.clone(),
            head_json,
            ManagedEntityKind::Npc,
            EntityId::from_bytes([NPC_BYTE; 16]),
        )
        .unwrap();
        let mut rival = project;
        rival.revision += 1;
        let prepared = selection
            .store
            .prepare_revision3_checkpoint(Some(&head), &rival)
            .unwrap();
        fs::write(temp.path().join("gore-project.json"), prepared.head_bytes).unwrap();
        assert_eq!(revalidate_store(&selection), Some(false));
        assert_eq!(
            closing_from_parts(Some(false), Some(false)),
            ClosingRevalidation::StoreAndGameDrift
        );
        assert_eq!(
            closing_from_parts(None, Some(true)),
            ClosingRevalidation::InspectionFailed
        );
    }

    #[test]
    fn evidence_response_is_strict_and_recovery_dominates_exact_current() {
        let project = npc_project(9);
        let (temp, head_json) = published_store(&project);
        let selection = open_initial_selection(
            temp.path().to_str().unwrap(),
            serde_json::from_str(&head_json).unwrap(),
            head_json.clone(),
            ManagedEntityKind::Npc,
            EntityId::from_bytes([NPC_BYTE; 16]),
        )
        .unwrap();
        let derived = DerivedModule {
            generated: selection.persisted_module.clone(),
        };
        let compiled = json!({
            "outcome": "compiled_evidence_only",
            "compile_error": Value::Null,
            "compiler_diagnostics": {
                "capture": "captured", "messages": [], "omitted": 0
            },
            "install_restore": "restored_exact",
            "recovery_required": false,
            "output_discarded": true,
        });
        let response = managed_response(&selection, &derived, compiled, true).unwrap();
        assert_eq!(response["ok"], true);
        assert_eq!(response["outcome"], "compiler_check_only");
        assert_eq!(response["exact_current"], true);
        assert_eq!(response["head_json"], head_json);
        assert_eq!(response["entity"]["kind"], "npc_draft");
        assert_eq!(response["entity"]["revision"], 2);
        assert_eq!(response["module"]["revision"], 3);
        assert!(response["compiler"].get("compiler_backend").is_none());
        assert_eq!(
            response["module"]["namespace"],
            "GoreMods.Npcs.ManagedCompilerCheck"
        );
        assert!(response["module"].get("source").is_none());
        assert!(response["compiler"].get("mini_path").is_none());
        assert!(response.get("closing_revalidation").is_none());
        for forbidden in [
            "build_ready",
            "deploy_ready",
            "artifact_path",
            "staging_path",
        ] {
            assert!(response.get(forbidden).is_none());
        }

        let recovery = preflight_compiler_evidence(
            "COMPILE_INSTALL_GUARD_RELEASE_FAILED",
            "retained recovery blocker",
            true,
        );
        let recovery_response = managed_response(&selection, &derived, recovery, true).unwrap();
        assert_eq!(recovery_response["outcome"], "compiler_check_only");
        assert_eq!(recovery_response["exact_current"], false);
        assert_eq!(recovery_response["compiler"]["recovery_required"], true);
        assert_eq!(recovery_response["module"]["id"], response["module"]["id"]);

        let residue = json!({
            "outcome": "failed",
            "compile_error": {"code": "COMPILE_OUTPUT_UNSAFE", "message": "discard failed"},
            "compiler_diagnostics": Value::Null,
            "install_restore": "restored_exact",
            "recovery_required": false,
            "output_discarded": false,
        });
        let residue_response = managed_response(&selection, &derived, residue, true).unwrap();
        assert_eq!(residue_response["exact_current"], true);
        assert_eq!(residue_response["compiler"]["outcome"], "failed");
        assert_eq!(residue_response["compiler"]["output_discarded"], false);
        assert_eq!(residue_response["compiler"]["recovery_required"], false);

        let sanitized = compiler_evidence(
            json!({
                "outcome": "compiled",
                "mini_path": "C:/must-not-escape/module.cache",
                "module": "GoreMods.MustNotEscape",
                "compile_error": Value::Null,
                "compiler_diagnostics": Value::Null,
                "install_restore": "restored_exact",
                "recovery_required": false,
            }),
            true,
        );
        assert_eq!(sanitized["outcome"], "compiled_evidence_only");
        assert!(sanitized.get("mini_path").is_none());
        assert!(sanitized.get("module").is_none());
        assert!(!sanitized.to_string().contains("must-not-escape"));
    }

    #[test]
    fn managed_compiler_evidence_redacts_private_paths_but_keeps_diagnostic_text() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path().join("SENTINEL_NATIVE_PRIVATE_PATH");
        let staging = workspace.join("gore-compile-sentinel");
        fs::create_dir_all(&staging).unwrap();
        let canonical_staging = staging.canonicalize().unwrap();
        let slash_staging = staging.to_string_lossy().replace('\\', "/");
        let upper_workspace = workspace.to_string_lossy().to_ascii_uppercase();

        let evidence = compiler_evidence_with_private_paths(
            json!({
                "outcome": "failed",
                "compile_error": {
                    "code": "COMPILE_IO",
                    "message": format!(
                        "ordinary compile detail while opening {}/tree/module.cache",
                        canonical_staging.display()
                    ),
                },
                "compiler_diagnostics": {
                    "capture": "captured",
                    "messages": [
                        {
                            "file": format!("{slash_staging}/tree/Managed.as"),
                            "line": 7,
                            "column": 3,
                            "severity": "error",
                            "message": "ordinary diagnostic text survives",
                        },
                        {
                            "file": "Managed.as",
                            "line": 8,
                            "column": 4,
                            "severity": "note",
                            "message": format!(
                                "ordinary note referencing {upper_workspace}\\gore-compile-sentinel"
                            ),
                        },
                    ],
                    "omitted": 0,
                },
                "install_restore": "restored_exact",
                "recovery_required": false,
            }),
            true,
            &[&workspace, &staging],
        );

        let encoded = serde_json::to_string(&evidence).unwrap();
        assert!(!encoded.contains("SENTINEL_NATIVE_PRIVATE_PATH"));
        assert!(!encoded.contains(&staging.to_string_lossy().into_owned()));
        assert!(!encoded.contains(&canonical_staging.to_string_lossy().into_owned()));
        assert!(encoded.contains(PRIVATE_PATH_REDACTION));
        assert!(encoded.contains("ordinary compile detail"));
        assert!(encoded.contains("ordinary diagnostic text survives"));
        assert!(encoded.contains("ordinary note referencing"));
    }

    #[test]
    fn managed_output_rejects_wrong_module_and_destroys_failed_residue() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path().join("workspace");
        let game = temp.path().join("game");
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&game).unwrap();
        let staging = OwnedCompileStaging::create(&workspace, &game).unwrap();
        let mini = staging.path().join("module.cache");

        fs::write(&mini, b"compiled bytes must not escape").unwrap();
        let mut compiled = CompileModuleReportOutcome::Compiled(
            gore_as::compile::CompileOutput::bind_existing(
                mini.clone(),
                "GoreMods.WrongModule".to_owned(),
            )
            .unwrap(),
        );
        let (discarded, rejection) =
            discard_managed_output(&staging, &mut compiled, "GoreMods.ExpectedModule");
        assert!(discarded);
        assert!(rejection.unwrap().contains("GoreMods.WrongModule"));
        assert!(!mini.exists() || fs::read(&mini).unwrap().is_empty());

        fs::write(&mini, b"partial failed compiler write").unwrap();
        let mut failed = CompileModuleReportOutcome::Failed(gore_as::compile::CompileError::Other(
            "injected failure".to_owned(),
        ));
        let (discarded, rejection) =
            discard_managed_output(&staging, &mut failed, "GoreMods.ExpectedModule");
        assert!(discarded);
        assert!(rejection.is_none());
        assert!(!mini.exists() || fs::read(&mini).unwrap().is_empty());
    }

    #[test]
    fn post_guard_preflight_failure_is_structured_and_releases_ownership() {
        let project = npc_project(11);
        let (temp, head_json) = published_store(&project);
        let selection = open_initial_selection(
            temp.path().to_str().unwrap(),
            serde_json::from_str(&head_json).unwrap(),
            head_json,
            ManagedEntityKind::Npc,
            EntityId::from_bytes([NPC_BYTE; 16]),
        )
        .unwrap();
        let derived = DerivedModule {
            generated: selection.persisted_module.clone(),
        };
        let game = temp.path().join("offline-game");
        fs::create_dir_all(&game).unwrap();
        // The stated answer is what keeps this test about the structured failure and the released
        // ownership. `gore-ffi` links `gore-as` compiled without `cfg(test)`, so the seam its own
        // fixtures use is out of reach here and the real process inspection would answer instead —
        // passing or failing on whether the developer happens to have Gothic open.
        let guard =
            acquire_compile_install_mutation_with_stated_game_process(&game, || Ok(false)).unwrap();
        let response = release_after_preflight(
            selection,
            guard,
            &derived,
            Failure::new(
                "AUTHORING_REVISION3_COMPILER_STAGING_UNAVAILABLE",
                "offline injected preflight failure",
            ),
        )
        .unwrap();
        assert_eq!(response["ok"], true);
        assert_eq!(response["outcome"], "compiler_check_only");
        assert_eq!(response["exact_current"], false);
        assert_eq!(response["compiler"]["outcome"], "failed");
        assert_eq!(response["compiler"]["install_restore"], "not_started");
        assert_eq!(response["compiler"]["recovery_required"], false);
        assert_eq!(response["compiler"]["output_discarded"], true);
        assert!(!game.join(".gore-install-mutation.lock").exists());
    }
}
