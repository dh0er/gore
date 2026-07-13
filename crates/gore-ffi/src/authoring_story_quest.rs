//! Closed native Quest-Draft transaction for one pristine game generation and exact project.
//!
//! The client supplies authoring intent only. Generation provenance, catalog selections, and the
//! complete collision inventory are rebuilt in memory from the selected game root. This command
//! never compiles, writes, deploys, publishes, or qualifies runtime behaviour.

use std::fs::File;
use std::io::{Read as _, Take};
use std::path::{Path, PathBuf};

use gore_authoring::{
    Diagnostic, DiagnosticCode, DiagnosticSeverity, EntityId, ProjectDocument, ProjectId,
    ProjectRevision2, QuestDraftCreateInput, Revision2EntityKind, Revision2EntityPayload,
    StoryDraftCreate, StoryDraftInsertEvaluation, StoryDraftInsertOutcome, StoryDraftInsertRequest,
    ValidationProfile, DRAFT_QUEST_GENERATOR_ID, DRAFT_QUEST_GENERATOR_VERSION,
    MAX_PROJECT_JSON_BYTES,
};
use gore_story_catalog::{
    build_known_catalog_with_shipping_snapshot, CatalogError, ContentSeal, GenerationInputLimits,
    StoryCatalogFile,
};
use gore_story_inventory::{
    build_base_game_inventory, QuestCollisionCapabilityError, StoryInventoryError,
    VerifiedQuestCollisionCapability, MAX_BINDS_CACHE_SOURCE_BYTES,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use sha2::{Digest as _, Sha256};

use crate::err;

const MAX_INTENT_JSON_BYTES: usize = 64 * 1024;
const MAX_GAME_ROOT_BYTES: usize = 32 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_DIAGNOSTICS: usize = 262_144;
const MAX_DIAGNOSTIC_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_DIAGNOSTIC_PROPERTY_PATH_BYTES: usize = 4 * 1024;
const MAX_DIAGNOSTIC_RELATED_ENTITIES: usize = 100_000;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_WIRE_REVISION: u64 = i64::MAX as u64 - 1;
const REQUEST_BINDING_DOMAIN: &[u8] =
    b"gore-ffi.authoring-project-story-quest-draft-insert-v1.request-binding\0";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QuestIntent {
    expected_project_id: ProjectId,
    expected_revision: u64,
    draft_id: EntityId,
    script_module_id: EntityId,
    display_name: String,
    module_namespace: String,
    technical_id: String,
    text_helper: String,
    title: String,
    description: String,
    objective_title: String,
    parent_catalog_id: String,
    giver_catalog_id: String,
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

pub(super) fn insert_quest_draft_v1(payload: Value) -> Value {
    insert_quest_draft_v1_inner(&payload, MAX_RESPONSE_BYTES).unwrap_or_else(Failure::response)
}

fn insert_quest_draft_v1_inner(payload: &Value, response_limit: usize) -> Result<Value, Failure> {
    let object = exact_payload(payload)?;
    let project_json = bounded_string(
        object,
        "project_json",
        MAX_PROJECT_JSON_BYTES,
        "AUTHORING_STORY_QUEST_PROJECT_LIMIT",
    )?;
    let intent_json = bounded_string(
        object,
        "intent_json",
        MAX_INTENT_JSON_BYTES,
        "AUTHORING_STORY_QUEST_INTENT_LIMIT",
    )?;
    let (profile, profile_wire) = closed_profile(object)?;
    let game_root = bounded_game_root(object)?;
    let request_binding_sha256 =
        request_binding(project_json, intent_json, profile_wire, game_root);

    let project = parse_exact_revision2_project(project_json)?;
    require_wire_revision(project.revision, "project revision")?;
    let intent = parse_intent(intent_json)?;
    require_wire_revision(intent.expected_revision, "expected revision")?;

    let game_root = PathBuf::from(game_root);
    let g1r = if game_root.file_name().is_some_and(|name| name == "G1R") {
        game_root.clone()
    } else {
        game_root.join("G1R")
    };
    let executable = g1r
        .join("Binaries")
        .join("Win64")
        .join("G1R-Win64-Shipping.exe");
    let binds_path = g1r.join("Script").join("Binds.Cache");

    // `pristine_script_cache` is the only Shipping-cache selector. It understands active deploy
    // records and drift, so neither the client nor this bridge may choose a live/backup cache.
    let shipping = gore_mod::pristine_script_cache(&game_root).map_err(map_pristine_error)?;
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
    let binds = read_bounded_regular(&binds_path, MAX_BINDS_CACHE_SOURCE_BYTES as u64)?;
    catalog
        .revalidate_generation_inputs()
        .map_err(map_catalog_error)?;
    let inventory =
        build_base_game_inventory(&catalog, &shipping, &binds).map_err(map_inventory_error)?;

    let capability = VerifiedQuestCollisionCapability::bind(inventory, &catalog, &project)
        .map_err(map_capability_error)?;
    let base_inventory_payload_seal = capability.base_inventory_payload_seal().clone();
    let combined_source_seal = capability.combined_source_seal().clone();
    let canonical_project_seal = capability.canonical_project().clone();
    let canonical_project_revision = capability.project_revision();
    let parent = capability
        .resolve_parent(&intent.parent_catalog_id)
        .map_err(map_capability_error)?;
    let giver = capability
        .resolve_giver(&intent.giver_catalog_id)
        .map_err(map_capability_error)?;
    let collision_catalog = capability
        .into_quest_collision_input(&project)
        .map_err(map_capability_error)?;

    let expected = ExpectedApplied {
        project_id: project.project_id,
        base_revision: project.revision,
        draft_id: intent.draft_id,
        script_module_id: intent.script_module_id,
        display_name: intent.display_name.clone(),
        module_namespace: intent.module_namespace.clone(),
        technical_id: intent.technical_id.clone(),
    };
    let request = StoryDraftInsertRequest {
        expected_project_id: intent.expected_project_id,
        expected_revision: intent.expected_revision,
        draft_id: intent.draft_id,
        script_module_id: intent.script_module_id,
        display_name: intent.display_name,
        draft: StoryDraftCreate::Quest(QuestDraftCreateInput {
            module_namespace: intent.module_namespace,
            technical_id: intent.technical_id,
            text_helper: intent.text_helper,
            parent_quest: parent,
            giver,
            title: intent.title,
            description: intent.description,
            objective_title: intent.objective_title,
            collision_catalog,
        }),
    };

    revalidate_sources(&catalog, &game_root, &shipping)?;
    let evaluation = project
        .insert_story_draft(request, profile)
        .map_err(|_| transaction_failed())?;
    revalidate_sources(&catalog, &game_root, &shipping)?;

    let provenance = ResponseProvenance {
        request_binding_sha256,
        base_inventory_payload_seal,
        combined_source_seal,
        canonical_project_seal,
        canonical_project_revision,
    };
    let response = response_for_evaluation(evaluation, expected, provenance, response_limit)?;
    // Close the final serialization window as well. `response_for_evaluation` serializes once to
    // enforce its exact budget before this last generation/pristine revalidation.
    revalidate_sources(&catalog, &game_root, &shipping)?;
    Ok(response)
}

struct ResponseProvenance {
    request_binding_sha256: String,
    base_inventory_payload_seal: ContentSeal,
    combined_source_seal: ContentSeal,
    canonical_project_seal: gore_authoring::ContentSeal,
    canonical_project_revision: u64,
}

struct ExpectedApplied {
    project_id: ProjectId,
    base_revision: u64,
    draft_id: EntityId,
    script_module_id: EntityId,
    display_name: String,
    module_namespace: String,
    technical_id: String,
}

fn response_for_evaluation(
    evaluation: StoryDraftInsertEvaluation,
    expected: ExpectedApplied,
    provenance: ResponseProvenance,
    response_limit: usize,
) -> Result<Value, Failure> {
    let ResponseProvenance {
        request_binding_sha256,
        base_inventory_payload_seal,
        combined_source_seal,
        canonical_project_seal,
        canonical_project_revision,
    } = provenance;
    let response = match evaluation {
        StoryDraftInsertEvaluation::Rejected(rejection) => {
            if rejection_is_quest_size_limit(&rejection.diagnostics) {
                return Err(Failure::new(
                    "AUTHORING_STORY_QUEST_PROJECT_LIMIT",
                    format!(
                        "one Quest Draft would exceed the {MAX_PROJECT_JSON_BYTES}-byte project limit"
                    ),
                ));
            }
            require_blocking_error(&rejection.diagnostics)?;
            let diagnostics = diagnostics_to_wire(rejection.diagnostics, response_limit, 2048)?;
            json!({
                "ok": true,
                "outcome": "rejected",
                "request_binding_sha256": request_binding_sha256,
                "diagnostics": diagnostics,
                "base_inventory_payload_seal": base_inventory_payload_seal,
                "combined_source_seal": combined_source_seal,
                "canonical_project_seal": canonical_project_seal,
                "canonical_project_revision": canonical_project_revision,
                "coverage": "base_game_and_exact_project_only",
                "runtime_qualification": "runtime_unqualified",
                "build_status": "blocked",
                "publication_status": "not_supported",
            })
        }
        StoryDraftInsertEvaluation::Applied(outcome) => {
            let outcome = *outcome;
            validate_applied(&outcome, &expected)?;
            if outcome.canonical_project_json.len() > MAX_PROJECT_JSON_BYTES {
                return Err(Failure::new(
                    "AUTHORING_STORY_QUEST_PROJECT_LIMIT",
                    format!(
                        "one Quest Draft would exceed the {MAX_PROJECT_JSON_BYTES}-byte project limit"
                    ),
                ));
            }
            if !outcome.blocks_build || !outcome.diagnostics.iter().any(is_combined_gate) {
                return Err(transaction_failed());
            }
            let reserved = outcome
                .canonical_project_json
                .len()
                .checked_add(4096)
                .ok_or_else(response_limit_failure)?;
            let diagnostics = diagnostics_to_wire(outcome.diagnostics, response_limit, reserved)?;
            json!({
                "ok": true,
                "outcome": "applied",
                "request_binding_sha256": request_binding_sha256,
                "project_json": outcome.canonical_project_json,
                "revision": outcome.project.revision,
                "draft_id": outcome.draft_id.to_string(),
                "draft_kind": "quest_draft",
                "script_module_id": outcome.script_module_id.to_string(),
                "diagnostics": diagnostics,
                "blocks_build": outcome.blocks_build,
                "base_inventory_payload_seal": base_inventory_payload_seal,
                "combined_source_seal": combined_source_seal,
                "canonical_project_seal": canonical_project_seal,
                "canonical_project_revision": canonical_project_revision,
                "coverage": "base_game_and_exact_project_only",
                "runtime_qualification": "runtime_unqualified",
                "build_status": "blocked",
                "publication_status": "not_supported",
            })
        }
    };
    enforce_response_budget(response, response_limit)
}

fn validate_applied(
    outcome: &StoryDraftInsertOutcome,
    expected: &ExpectedApplied,
) -> Result<(), Failure> {
    let revision = expected
        .base_revision
        .checked_add(1)
        .ok_or_else(transaction_failed)?;
    if outcome.project.project_id != expected.project_id
        || outcome.project.revision != revision
        || outcome.draft_id != expected.draft_id
        || outcome.script_module_id != expected.script_module_id
        || outcome.draft_kind != Revision2EntityKind::QuestDraft
    {
        return Err(transaction_failed());
    }
    let draft = outcome
        .project
        .entities
        .get(&expected.draft_id)
        .ok_or_else(transaction_failed)?;
    let Revision2EntityPayload::QuestDraft(quest) = &draft.payload else {
        return Err(transaction_failed());
    };
    if draft.display_name != expected.display_name
        || quest.generator_id != DRAFT_QUEST_GENERATOR_ID
        || quest.generator_version != DRAFT_QUEST_GENERATOR_VERSION
        || quest.input.quest_id != expected.draft_id
        || quest.input.module_namespace != expected.module_namespace
        || quest.input.technical_id != expected.technical_id
        || quest.script_module.id != expected.script_module_id
    {
        return Err(transaction_failed());
    }
    let reopened = ProjectRevision2::from_json(&outcome.canonical_project_json)
        .map_err(|_| transaction_failed())?;
    if reopened != outcome.project {
        return Err(transaction_failed());
    }
    Ok(())
}

fn exact_payload(payload: &Value) -> Result<&Map<String, Value>, Failure> {
    let object = payload.as_object().ok_or_else(invalid_request)?;
    if object.len() != 4
        || !object.contains_key("project_json")
        || !object.contains_key("intent_json")
        || !object.contains_key("profile")
        || !object.contains_key("game_root")
    {
        return Err(invalid_request());
    }
    Ok(object)
}

fn bounded_string<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    max: usize,
    limit_code: &'static str,
) -> Result<&'a str, Failure> {
    let value = object
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(invalid_request)?;
    if value.len() > max {
        return Err(Failure::new(
            limit_code,
            format!("'{field}' exceeds its {max}-byte limit"),
        ));
    }
    Ok(value)
}

fn bounded_game_root(object: &Map<String, Value>) -> Result<&str, Failure> {
    object
        .get("game_root")
        .and_then(Value::as_str)
        .filter(|value| {
            !value.is_empty() && value.len() <= MAX_GAME_ROOT_BYTES && !value.contains('\0')
        })
        .ok_or_else(invalid_request)
}

fn closed_profile(object: &Map<String, Value>) -> Result<(ValidationProfile, &str), Failure> {
    match object.get("profile").and_then(Value::as_str) {
        Some("production") => Ok((ValidationProfile::Production, "production")),
        Some("experimental") => Ok((ValidationProfile::Experimental, "experimental")),
        _ => Err(invalid_request()),
    }
}

fn parse_exact_revision2_project(project_json: &str) -> Result<ProjectRevision2, Failure> {
    let document = ProjectDocument::from_json(project_json).map_err(|_| {
        Failure::new(
            "AUTHORING_STORY_QUEST_PROJECT_INVALID",
            "project_json is not a valid closed authoring project",
        )
    })?;
    let canonical = document.to_canonical_json().map_err(|_| {
        Failure::new(
            "AUTHORING_STORY_QUEST_PROJECT_INVALID",
            "project_json could not be serialized canonically",
        )
    })?;
    if canonical != project_json {
        return Err(Failure::new(
            "AUTHORING_STORY_QUEST_PROJECT_NONCANONICAL",
            "project_json is not in exact canonical encoding",
        ));
    }
    match document {
        ProjectDocument::Revision2(project) => Ok(project),
        ProjectDocument::Revision1(_) => Err(Failure::new(
            "AUTHORING_STORY_QUEST_PROJECT_REVISION_REQUIRED",
            "Quest Draft insertion requires schema revision 2",
        )),
    }
}

fn parse_intent(intent_json: &str) -> Result<QuestIntent, Failure> {
    serde_json::from_str(intent_json).map_err(|_| {
        Failure::new(
            "AUTHORING_STORY_QUEST_INTENT_INVALID",
            "intent_json must contain exactly the closed Quest intent fields",
        )
    })
}

fn request_binding(
    project_json: &str,
    intent_json: &str,
    profile: &str,
    game_root: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(REQUEST_BINDING_DOMAIN);
    for bytes in [
        project_json.as_bytes(),
        intent_json.as_bytes(),
        profile.as_bytes(),
        game_root.as_bytes(),
    ] {
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    hex_digest(hasher.finalize())
}

fn revalidate_sources(
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
        return Err(Failure::new(
            "AUTHORING_STORY_QUEST_INPUT_CHANGED",
            "the native game generation changed during Quest insertion",
        ));
    }
    catalog
        .revalidate_generation_inputs()
        .map_err(map_catalog_error)
}

fn read_bounded_regular(path: &Path, max_bytes: u64) -> Result<Vec<u8>, Failure> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| source_unavailable())?;
    if !metadata.file_type().is_file() || metadata_is_link(&metadata) {
        return Err(Failure::new(
            "AUTHORING_STORY_QUEST_UNSAFE_INPUT",
            "a fixed native generation input is not a safe regular file",
        ));
    }
    if metadata.len() == 0 || metadata.len() > max_bytes {
        return Err(Failure::new(
            "AUTHORING_STORY_QUEST_INPUT_LIMIT",
            "a fixed native generation input exceeds its resource limit",
        ));
    }
    let mut file = File::open(path).map_err(|_| source_unavailable())?;
    let capacity = usize::try_from(metadata.len()).map_err(|_| source_limit())?;
    let mut bytes = Vec::with_capacity(capacity);
    let mut reader: Take<&mut File> = file.by_ref().take(max_bytes.saturating_add(1));
    reader
        .read_to_end(&mut bytes)
        .map_err(|_| source_unavailable())?;
    if bytes.len() as u64 != metadata.len() || bytes.len() as u64 > max_bytes {
        return Err(Failure::new(
            "AUTHORING_STORY_QUEST_INPUT_CHANGED",
            "a fixed native generation input changed while it was read",
        ));
    }
    Ok(bytes)
}

fn metadata_is_link(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

fn diagnostics_to_wire(
    diagnostics: Vec<Diagnostic>,
    response_limit: usize,
    mut estimated_bytes: usize,
) -> Result<Vec<Value>, Failure> {
    if diagnostics.len() > MAX_DIAGNOSTICS {
        return Err(response_limit_failure());
    }
    let mut wire = Vec::with_capacity(diagnostics.len().min(1024));
    for diagnostic in diagnostics {
        if diagnostic
            .property_path
            .as_ref()
            .is_some_and(|path| path.len() > MAX_DIAGNOSTIC_PROPERTY_PATH_BYTES)
            || diagnostic.related_entities.len() > MAX_DIAGNOSTIC_RELATED_ENTITIES
        {
            return Err(response_limit_failure());
        }
        let message = truncate_utf8(diagnostic.message, MAX_DIAGNOSTIC_MESSAGE_BYTES);
        let item = json!({
            "code": diagnostic.code,
            "severity": diagnostic.severity,
            "entity": diagnostic.entity.map(|entity| entity.to_string()),
            "property_path": diagnostic.property_path,
            "message": message,
            "related_entities": diagnostic.related_entities.into_iter()
                .map(|entity| entity.to_string()).collect::<Vec<_>>(),
            "blocks_build": diagnostic.blocks_build,
        });
        let item_len = serde_json::to_vec(&item)
            .map_err(|_| response_limit_failure())?
            .len();
        estimated_bytes = estimated_bytes
            .checked_add(item_len + 1)
            .filter(|total| *total <= response_limit)
            .ok_or_else(response_limit_failure)?;
        wire.push(item);
    }
    Ok(wire)
}

fn rejection_is_quest_size_limit(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::InvalidStoryMutation
            && diagnostic.blocks_build
            && (diagnostic.property_path.as_deref() == Some("project_json")
                || (diagnostic.property_path.as_deref() == Some("project")
                    && diagnostic
                        .message
                        .contains("working-store resource limit exceeded")))
    })
}

fn require_blocking_error(diagnostics: &[Diagnostic]) -> Result<(), Failure> {
    if diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == DiagnosticSeverity::Error && diagnostic.blocks_build
    }) {
        Ok(())
    } else {
        Err(transaction_failed())
    }
}

fn is_combined_gate(diagnostic: &Diagnostic) -> bool {
    diagnostic.code == DiagnosticCode::Revision2CombinedValidationUnavailable
        && diagnostic.severity == DiagnosticSeverity::Error
        && diagnostic.entity.is_none()
        && diagnostic.property_path.as_deref() == Some("schema_revision")
        && diagnostic.blocks_build
}

fn enforce_response_budget(response: Value, limit: usize) -> Result<Value, Failure> {
    let bytes = serde_json::to_vec(&response).map_err(|_| response_limit_failure())?;
    if bytes.len() > limit {
        return Err(response_limit_failure());
    }
    Ok(response)
}

fn require_wire_revision(revision: u64, kind: &str) -> Result<(), Failure> {
    if revision <= MAX_WIRE_REVISION {
        Ok(())
    } else {
        Err(Failure::new(
            "AUTHORING_STORY_QUEST_REVISION_WIRE_LIMIT",
            format!("{kind} exceeds the signed 64-bit native wire limit"),
        ))
    }
}

fn invalid_request() -> Failure {
    Failure::new(
        "AUTHORING_STORY_QUEST_REQUEST_INVALID",
        "payload must contain exactly project_json, intent_json, profile, and game_root",
    )
}

fn source_unavailable() -> Failure {
    Failure::new(
        "AUTHORING_STORY_QUEST_INPUT_UNAVAILABLE",
        "a required fixed native generation input is unavailable",
    )
}

fn source_limit() -> Failure {
    Failure::new(
        "AUTHORING_STORY_QUEST_INPUT_LIMIT",
        "a fixed native generation input exceeds its resource limit",
    )
}

fn transaction_failed() -> Failure {
    Failure::new(
        "AUTHORING_STORY_QUEST_TRANSACTION_FAILED",
        "the closed atomic Quest Draft transaction failed",
    )
}

fn response_limit_failure() -> Failure {
    Failure::new(
        "AUTHORING_STORY_QUEST_RESPONSE_LIMIT",
        "the Quest Draft transaction response exceeds its bounded transport budget",
    )
}

fn map_pristine_error(error: gore_mod::ModError) -> Failure {
    let message = error.to_string();
    if message.contains("RECOVERY_REQUIRED") {
        return Failure::new(
            "AUTHORING_STORY_QUEST_RECOVERY_REQUIRED",
            "an interrupted deployment must be recovered before Quest authoring",
        );
    }
    if message.contains("exceeds the") || message.contains("too large") {
        return source_limit();
    }
    if message.contains("not a regular non-link file") {
        return Failure::new(
            "AUTHORING_STORY_QUEST_UNSAFE_INPUT",
            "the pristine Shipping cache is not a safe regular file",
        );
    }
    Failure::new(
        "AUTHORING_STORY_QUEST_PRISTINE_UNAVAILABLE",
        "the pristine Shipping cache could not be selected safely",
    )
}

fn map_catalog_error(error: CatalogError) -> Failure {
    match error {
        CatalogError::InvalidLimits(_) | CatalogError::LimitExceeded { .. } => source_limit(),
        CatalogError::UnsafeInput(_) | CatalogError::OutputAliasesInput { .. } => Failure::new(
            "AUTHORING_STORY_QUEST_UNSAFE_INPUT",
            "a fixed native generation input is unsafe",
        ),
        CatalogError::IdentityChanged(_) | CatalogError::SourceChanged { .. } => Failure::new(
            "AUTHORING_STORY_QUEST_INPUT_CHANGED",
            "the native game generation changed during Quest insertion",
        ),
        CatalogError::UnsupportedGeneration { .. } => Failure::new(
            "AUTHORING_STORY_QUEST_UNSUPPORTED_GENERATION",
            "the selected game does not match the supported pinned generation",
        ),
        _ => source_unavailable(),
    }
}

fn map_inventory_error(error: StoryInventoryError) -> Failure {
    match error {
        StoryInventoryError::LimitExceeded { .. } | StoryInventoryError::SourcePairTooLarge => {
            source_limit()
        }
        StoryInventoryError::UnsupportedGeneration => Failure::new(
            "AUTHORING_STORY_QUEST_UNSUPPORTED_GENERATION",
            "the selected game does not match the supported pinned generation",
        ),
        StoryInventoryError::SourceLengthMismatch { .. }
        | StoryInventoryError::SourceDigestMismatch { .. }
        | StoryInventoryError::SourcePairSealMismatch
        | StoryInventoryError::RecollectedInventoryMismatch => Failure::new(
            "AUTHORING_STORY_QUEST_INPUT_CHANGED",
            "the native game generation changed during Quest insertion",
        ),
        _ => Failure::new(
            "AUTHORING_STORY_QUEST_INVENTORY_FAILED",
            "the closed base-game collision inventory could not be rebuilt",
        ),
    }
}

fn map_capability_error(error: QuestCollisionCapabilityError) -> Failure {
    match error {
        QuestCollisionCapabilityError::UnknownParent(_) => Failure::new(
            "AUTHORING_STORY_QUEST_PARENT_UNKNOWN",
            "parent_catalog_id is not present in the trusted Story catalog",
        ),
        QuestCollisionCapabilityError::UnknownGiver(_) => Failure::new(
            "AUTHORING_STORY_QUEST_GIVER_UNKNOWN",
            "giver_catalog_id is not present in the trusted Story catalog",
        ),
        QuestCollisionCapabilityError::TargetMismatch => Failure::new(
            "AUTHORING_STORY_QUEST_PROJECT_TARGET_MISMATCH",
            "the exact project does not target the selected trusted game generation",
        ),
        QuestCollisionCapabilityError::Limit { .. } => Failure::new(
            "AUTHORING_STORY_QUEST_COLLISION_LIMIT",
            "base-game and exact-project collision identities exceed the supported limit",
        ),
        QuestCollisionCapabilityError::ProjectDrift => Failure::new(
            "AUTHORING_STORY_QUEST_PROJECT_CHANGED",
            "the exact canonical project changed while collision capability was bound",
        ),
        _ => Failure::new(
            "AUTHORING_STORY_QUEST_CAPABILITY_FAILED",
            "the closed Quest collision capability could not be bound",
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

fn hex_digest(digest: impl IntoIterator<Item = u8>) -> String {
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use gore_story_catalog::known_generation_v1;

    fn project_json() -> String {
        let generation = known_generation_v1();
        let raw = json!({
            "format": 2,
            "schema_revision": 2,
            "project_id": "01010101010101010101010101010101",
            "revision": 7,
            "meta": {"name": "Quest MVP", "version": "0.1", "author": "tests"},
            "target": {"executable": generation.executable},
            "authoring_locales": [],
            "entities": {},
            "asset_store": {"assets": {}}
        })
        .to_string();
        ProjectDocument::from_json(&raw)
            .unwrap()
            .to_canonical_json()
            .unwrap()
    }

    fn intent_value() -> Value {
        json!({
            "expected_project_id": "01010101010101010101010101010101",
            "expected_revision": 7,
            "draft_id": "10101010101010101010101010101010",
            "script_module_id": "11111111111111111111111111111111",
            "display_name": "Asghan's Trial",
            "module_namespace": "GoreMods.Quests.AsghanTrial",
            "technical_id": "GORE_ASGHAN_TRIAL",
            "text_helper": "GoreAsghanTrialText",
            "title": "Asghan's Trial",
            "description": "Report to Asghan.",
            "objective_title": "Speak to Asghan",
            "parent_catalog_id": "g1r:quest-parent:swampcamp_scchapter2",
            "giver_catalog_id": "g1r:npc:om_grd_asghan_263"
        })
    }

    fn payload() -> Value {
        json!({
            "project_json": project_json(),
            "intent_json": intent_value().to_string(),
            "profile": "experimental",
            "game_root": "C:/missing/game"
        })
    }

    #[test]
    fn payload_and_intent_are_exact_and_provenance_free() {
        assert!(exact_payload(&payload()).is_ok());
        for invalid in [
            Value::Null,
            json!({}),
            json!({"project_json": "x", "intent_json": "x", "profile": "experimental"}),
            {
                let mut value = payload();
                value["inventory_json"] = json!("forbidden");
                value
            },
        ] {
            assert_eq!(
                insert_quest_draft_v1(invalid)["error"]["code"],
                "AUTHORING_STORY_QUEST_REQUEST_INVALID"
            );
        }

        let parsed = parse_intent(&intent_value().to_string()).unwrap();
        assert_eq!(parsed.technical_id, "GORE_ASGHAN_TRIAL");
        for forbidden in [
            "source_seal",
            "inventory_json",
            "collision_catalog",
            "modules",
            "relative_paths",
            "symbols",
        ] {
            let mut intent = intent_value();
            intent[forbidden] = json!("forbidden");
            assert!(parse_intent(&intent.to_string()).is_err());
        }
    }

    #[test]
    fn duplicate_and_oversized_intent_fail_before_native_io() {
        let intent = intent_value().to_string();
        let duplicate = intent.replacen(
            "\"expected_revision\":7",
            "\"expected_revision\":7,\"expected_revision\":7",
            1,
        );
        let mut request = payload();
        request["intent_json"] = json!(duplicate);
        assert_eq!(
            insert_quest_draft_v1(request)["error"]["code"],
            "AUTHORING_STORY_QUEST_INTENT_INVALID"
        );

        let mut request = payload();
        request["intent_json"] = json!("x".repeat(MAX_INTENT_JSON_BYTES + 1));
        assert_eq!(
            insert_quest_draft_v1(request)["error"]["code"],
            "AUTHORING_STORY_QUEST_INTENT_LIMIT"
        );
    }

    #[test]
    fn request_binding_covers_every_raw_component() {
        let base = request_binding("project", "intent", "experimental", "game");
        for changed in [
            request_binding("project2", "intent", "experimental", "game"),
            request_binding("project", "intent2", "experimental", "game"),
            request_binding("project", "intent", "production", "game"),
            request_binding("project", "intent", "experimental", "game2"),
        ] {
            assert_ne!(base, changed);
        }
        assert_eq!(base.len(), 64);
    }

    #[test]
    fn missing_game_errors_are_sanitized() {
        let request = payload();
        let root = request["game_root"].as_str().unwrap().to_owned();
        let response = insert_quest_draft_v1(request);
        assert_eq!(response["ok"], false);
        assert!(!response.to_string().contains(&root));
    }

    #[test]
    fn command_is_advertised_and_dispatches_through_the_native_protocol() {
        let info: Value = serde_json::from_str(&crate::execute_json(
            r#"{"command":"core_info","payload":{}}"#,
        ))
        .unwrap();
        assert!(info["commands"]
            .as_array()
            .unwrap()
            .contains(&json!("authoring_project_story_quest_draft_insert_v1")));

        let root = payload()["game_root"].as_str().unwrap().to_owned();
        let request = json!({
            "command": "authoring_project_story_quest_draft_insert_v1",
            "payload": payload(),
        });
        let response: Value =
            serde_json::from_str(&crate::execute_json(&request.to_string())).unwrap();
        assert_eq!(response["ok"], false);
        assert!(!response.to_string().contains(&root));
    }

    #[test]
    fn project_limit_diagnostic_has_explicit_stable_classification() {
        let diagnostic = Diagnostic {
            code: DiagnosticCode::InvalidStoryMutation,
            severity: DiagnosticSeverity::Error,
            entity: None,
            property_path: Some("project_json".to_owned()),
            message: "too large".to_owned(),
            related_entities: Vec::new(),
            blocks_build: true,
        };
        assert!(rejection_is_quest_size_limit(&[diagnostic]));
    }

    #[test]
    #[ignore = "requires the configured pinned game generation"]
    fn configured_real_game_reaches_explicit_bounded_quest_limit() {
        let game_root = std::env::var("GORE_STORY_GAME_ROOT")
            .expect("set GORE_STORY_GAME_ROOT to the pinned game installation");
        let project_json = project_json();
        let intent_json = intent_value().to_string();
        let response = insert_quest_draft_v1(json!({
            "project_json": project_json,
            "intent_json": intent_json,
            "profile": "experimental",
            "game_root": game_root,
        }));
        assert_eq!(response["ok"], false, "{response}");
        assert_eq!(
            response["error"]["code"], "AUTHORING_STORY_QUEST_PROJECT_LIMIT",
            "{response}"
        );
        assert!(response.get("project_json").is_none());
    }
}
