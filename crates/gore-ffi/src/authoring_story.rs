//! Bounded raw-JSON bridge for atomic schema-revision-2 Story Draft insertion.

use std::collections::BTreeSet;

use gore_authoring::{
    Diagnostic, DiagnosticCode, DiagnosticSeverity, EntityId, LOGICAL_NPC_CLONE_GENERATOR_ID,
    LOGICAL_NPC_CLONE_GENERATOR_VERSION, MAX_PROJECT_JSON_BYTES, MAX_STORY_DRAFT_INSERT_JSON_BYTES,
    ProjectDocument, ProjectDocumentError, ProjectId, Revision2EntityKind, Revision2EntityPayload,
    Revision2OriginRef, StoryDraftCreate, StoryDraftInsertError, StoryDraftInsertEvaluation,
    StoryDraftInsertJsonError, StoryDraftInsertOutcome, StoryDraftInsertRequest, ValidationProfile,
    story_draft_insert_request_binding_sha256,
};
use serde_json::{Map, Value, json};

use crate::err;

const MAX_AUTHORING_STORY_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
// This FFI command intentionally exposes only signed-64-bit JSON revisions because Dart's native
// `int` wire decoder must reproduce them exactly. The core transaction remains `u64` throughout.
const MAX_AUTHORING_STORY_BASE_REVISION: u64 = i64::MAX as u64 - 1;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_DIAGNOSTICS: usize = 262_144;
const MAX_DIAGNOSTIC_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_DIAGNOSTIC_PROPERTY_PATH_BYTES: usize = 4 * 1024;
const MAX_DIAGNOSTIC_RELATED_ENTITIES: usize = 100_000;

#[derive(Debug)]
struct StoryFailure {
    code: &'static str,
    message: String,
}

struct ExpectedAppliedOutcome {
    project_id: ProjectId,
    base_revision: u64,
    draft_id: EntityId,
    script_module_id: EntityId,
    display_name: String,
    draft_kind: Revision2EntityKind,
    module_namespace: String,
    runtime_id: String,
    generator_id: &'static str,
    generator_version: u32,
}

impl StoryFailure {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: truncate_utf8_with_suffix(message.into(), MAX_ERROR_MESSAGE_BYTES, "..."),
        }
    }

    fn response(self) -> Value {
        err(self.code, self.message)
    }
}

pub(super) fn insert_story_draft_v1(payload: Value) -> Value {
    command_response(insert_story_draft_v1_inner(
        &payload,
        MAX_AUTHORING_STORY_RESPONSE_BYTES,
    ))
}

fn command_response(result: Result<Value, StoryFailure>) -> Value {
    match result {
        Ok(response) => response,
        Err(error) => error.response(),
    }
}

fn insert_story_draft_v1_inner(
    payload: &Value,
    response_limit: usize,
) -> Result<Value, StoryFailure> {
    let object = exact_payload(payload, &["mutation_json", "profile", "project_json"])?;
    let project_json = required_bounded_string(
        object,
        "project_json",
        MAX_PROJECT_JSON_BYTES,
        "AUTHORING_STORY_PROJECT_LIMIT",
        "authoring project JSON",
    )?;
    let mutation_json = required_bounded_string(
        object,
        "mutation_json",
        MAX_STORY_DRAFT_INSERT_JSON_BYTES,
        "AUTHORING_STORY_MUTATION_LIMIT",
        "story mutation JSON",
    )?;
    let profile = required_profile(object)?;
    let request_binding =
        story_draft_insert_request_binding_sha256(project_json, mutation_json, profile);

    // Both nested strings remain untouched. Parsing either through Value first would erase the
    // duplicate-key evidence that the closed authoring parsers deliberately reject.
    let document = ProjectDocument::from_json(project_json).map_err(map_project_error)?;
    let canonical = document.to_canonical_json().map_err(|_| {
        StoryFailure::new(
            "AUTHORING_STORY_PROJECT_INVALID",
            "canonical authoring project serialization failed",
        )
    })?;
    if canonical != project_json {
        return Err(StoryFailure::new(
            "AUTHORING_STORY_PROJECT_NONCANONICAL",
            "authoring project JSON is not in canonical revision-2 encoding",
        ));
    }
    let (base_project_id, base_revision) = match &document {
        ProjectDocument::Revision2(project) => (project.project_id, project.revision),
        ProjectDocument::Revision1(_) | ProjectDocument::Revision3(_) => {
            return Err(StoryFailure::new(
                "AUTHORING_STORY_PROJECT_REVISION_REQUIRED",
                "story Draft insertion requires schema revision 2",
            ));
        }
    };
    require_revision_wire_range(base_revision, "base project revision")?;

    let request = StoryDraftInsertRequest::from_json(mutation_json).map_err(map_mutation_error)?;
    require_revision_wire_range(request.expected_revision, "expected mutation revision")?;
    let (draft_kind, module_namespace, runtime_id, generator_id, generator_version) =
        match &request.draft {
            StoryDraftCreate::Npc(input) => (
                Revision2EntityKind::NpcDraft,
                input.module_namespace.clone(),
                input.unique_name.clone(),
                LOGICAL_NPC_CLONE_GENERATOR_ID,
                LOGICAL_NPC_CLONE_GENERATOR_VERSION,
            ),
            StoryDraftCreate::Quest(_) => {
                return Err(StoryFailure::new(
                    "AUTHORING_STORY_QUEST_INVENTORY_REQUIRED",
                    "Studio Quest insertion requires an exact trusted collision inventory",
                ));
            }
        };
    let expected = ExpectedAppliedOutcome {
        project_id: base_project_id,
        base_revision,
        draft_id: request.draft_id,
        script_module_id: request.script_module_id,
        display_name: request.display_name.clone(),
        draft_kind,
        module_namespace,
        runtime_id,
        generator_id,
        generator_version,
    };
    let evaluation = document
        .insert_story_draft(request, profile)
        .map_err(map_transaction_error)?;

    match evaluation {
        StoryDraftInsertEvaluation::Rejected(rejection) => {
            if !rejection.diagnostics.iter().any(|diagnostic| {
                diagnostic.severity == DiagnosticSeverity::Error && diagnostic.blocks_build
            }) {
                return Err(StoryFailure::new(
                    "AUTHORING_STORY_TRANSACTION_FAILED",
                    "story transaction rejected without a blocking error diagnostic",
                ));
            }
            let diagnostics = diagnostics_to_wire(rejection.diagnostics, response_limit, 256)?;
            let response = json!({
                "ok": true,
                "outcome": "rejected",
                "request_binding_sha256": request_binding.to_string(),
                "diagnostics": diagnostics,
            });
            enforce_response_budget(response, response_limit)
        }
        StoryDraftInsertEvaluation::Applied(outcome) => {
            let outcome = *outcome;
            validate_applied_outcome(&outcome, &expected)?;
            if outcome.canonical_project_json.len() > MAX_PROJECT_JSON_BYTES {
                return Err(StoryFailure::new(
                    "AUTHORING_STORY_RESPONSE_LIMIT",
                    format!(
                        "story transaction project JSON exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"
                    ),
                ));
            }
            if !outcome.blocks_build || !outcome.diagnostics.iter().any(is_combined_gate) {
                return Err(StoryFailure::new(
                    "AUTHORING_STORY_TRANSACTION_FAILED",
                    "applied revision-2 story transaction omitted its blocking combined-validation gate",
                ));
            }
            let draft_kind = match outcome.draft_kind {
                Revision2EntityKind::NpcDraft => "npc_draft",
                Revision2EntityKind::QuestDraft => "quest_draft",
                _ => {
                    return Err(StoryFailure::new(
                        "AUTHORING_STORY_TRANSACTION_FAILED",
                        "story transaction returned a non-Draft entity kind",
                    ));
                }
            };
            let reserved = outcome
                .canonical_project_json
                .len()
                .checked_add(1024)
                .ok_or_else(|| {
                    StoryFailure::new(
                        "AUTHORING_STORY_RESPONSE_LIMIT",
                        "story transaction response size overflow",
                    )
                })?;
            let diagnostics = diagnostics_to_wire(outcome.diagnostics, response_limit, reserved)?;
            let response = json!({
                "ok": true,
                "outcome": "applied",
                "request_binding_sha256": request_binding.to_string(),
                "project_json": outcome.canonical_project_json,
                "revision": outcome.project.revision,
                "draft_id": outcome.draft_id.to_string(),
                "draft_kind": draft_kind,
                "script_module_id": outcome.script_module_id.to_string(),
                "diagnostics": diagnostics,
                "blocks_build": outcome.blocks_build,
            });
            enforce_response_budget(response, response_limit)
        }
    }
}

fn require_revision_wire_range(revision: u64, kind: &str) -> Result<(), StoryFailure> {
    if revision <= MAX_AUTHORING_STORY_BASE_REVISION {
        Ok(())
    } else {
        Err(StoryFailure::new(
            "AUTHORING_STORY_REVISION_WIRE_LIMIT",
            format!(
                "{kind} exceeds the signed 64-bit Story Draft insertion wire limit of {MAX_AUTHORING_STORY_BASE_REVISION}"
            ),
        ))
    }
}

fn validate_applied_outcome(
    outcome: &StoryDraftInsertOutcome,
    expected: &ExpectedAppliedOutcome,
) -> Result<(), StoryFailure> {
    let expected_revision = expected.base_revision.checked_add(1).ok_or_else(|| {
        transaction_invariant("an applied story transaction overflowed the base revision")
    })?;
    if outcome.project.project_id != expected.project_id
        || outcome.project.revision != expected_revision
        || outcome.draft_id != expected.draft_id
        || outcome.script_module_id != expected.script_module_id
        || outcome.draft_kind != expected.draft_kind
    {
        return Err(transaction_invariant(
            "applied story transaction metadata does not match its exact base and request",
        ));
    }

    let draft = outcome
        .project
        .entities
        .get(&expected.draft_id)
        .ok_or_else(|| transaction_invariant("applied story Draft entity is missing"))?;
    if draft.id != expected.draft_id
        || draft.display_name != expected.display_name
        || draft.revision != 0
    {
        return Err(transaction_invariant(
            "applied story Draft metadata does not match the request",
        ));
    }
    match &draft.origin {
        Revision2OriginRef::New {
            authored_runtime_id,
        } if authored_runtime_id == &expected.runtime_id => {}
        _ => {
            return Err(transaction_invariant(
                "applied story Draft origin does not match the requested runtime identity",
            ));
        }
    }

    let script_ref = match (&draft.payload, expected.draft_kind) {
        (Revision2EntityPayload::NpcDraft(draft), Revision2EntityKind::NpcDraft) => {
            if draft.generator_id != expected.generator_id
                || draft.generator_version != expected.generator_version
            {
                return Err(transaction_invariant(
                    "applied NPC Draft generator does not match the closed contract",
                ));
            }
            &draft.script_module
        }
        (Revision2EntityPayload::QuestDraft(draft), Revision2EntityKind::QuestDraft) => {
            if draft.generator_id != expected.generator_id
                || draft.generator_version != expected.generator_version
                || draft.input.quest_id != expected.draft_id
            {
                return Err(transaction_invariant(
                    "applied Quest Draft identity or generator does not match the request",
                ));
            }
            &draft.script_module
        }
        _ => {
            return Err(transaction_invariant(
                "applied story Draft payload kind does not match the request",
            ));
        }
    };
    if script_ref.project_id != expected.project_id
        || script_ref.id != expected.script_module_id
        || script_ref.expected_kind != Revision2EntityKind::ScriptModule
    {
        return Err(transaction_invariant(
            "applied story Draft does not reference its exact generated ScriptModule",
        ));
    }

    let module = outcome
        .project
        .entities
        .get(&expected.script_module_id)
        .ok_or_else(|| transaction_invariant("applied ScriptModule entity is missing"))?;
    if module.id != expected.script_module_id
        || module.display_name != expected.module_namespace
        || module.revision != 0
    {
        return Err(transaction_invariant(
            "applied ScriptModule metadata does not match the request",
        ));
    }
    let origin_owner = match &module.origin {
        Revision2OriginRef::Generated {
            generator_id,
            generator_version,
            owner,
        } if generator_id == expected.generator_id
            && *generator_version == expected.generator_version =>
        {
            owner
        }
        _ => {
            return Err(transaction_invariant(
                "applied ScriptModule origin does not match the closed generator contract",
            ));
        }
    };
    let Revision2EntityPayload::ScriptModule(module_payload) = &module.payload else {
        return Err(transaction_invariant(
            "applied generated entity is not a ScriptModule payload",
        ));
    };
    if module_payload.generator_id != expected.generator_id
        || module_payload.generator_version != expected.generator_version
        || module_payload.module_namespace != expected.module_namespace
        || module_payload.module_relative_path
            != format!("{}.as", expected.module_namespace.replace('.', "/"))
        || module_payload.owner != *origin_owner
        || origin_owner.project_id != expected.project_id
        || origin_owner.id != expected.draft_id
        || origin_owner.expected_kind != expected.draft_kind
    {
        return Err(transaction_invariant(
            "applied ScriptModule origin and payload ownership are not bidirectionally exact",
        ));
    }
    Ok(())
}

fn is_combined_gate(diagnostic: &Diagnostic) -> bool {
    diagnostic.code == DiagnosticCode::Revision2CombinedValidationUnavailable
        && diagnostic.severity == DiagnosticSeverity::Error
        && diagnostic.entity.is_none()
        && diagnostic.property_path.as_deref() == Some("schema_revision")
        && diagnostic.blocks_build
}

fn transaction_invariant(message: &'static str) -> StoryFailure {
    StoryFailure::new("AUTHORING_STORY_TRANSACTION_FAILED", message)
}

fn exact_payload<'a>(
    payload: &'a Value,
    expected_fields: &[&str],
) -> Result<&'a Map<String, Value>, StoryFailure> {
    let object = payload.as_object().ok_or_else(|| {
        StoryFailure::new(
            "AUTHORING_STORY_PAYLOAD_INVALID",
            "payload must be an object",
        )
    })?;
    let expected = expected_fields.iter().copied().collect::<BTreeSet<_>>();
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(StoryFailure::new(
            "AUTHORING_STORY_PAYLOAD_INVALID",
            format!(
                "payload fields must be exactly: {}",
                expected_fields.join(", ")
            ),
        ));
    }
    Ok(object)
}

fn required_bounded_string<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    max_bytes: usize,
    limit_code: &'static str,
    kind: &str,
) -> Result<&'a str, StoryFailure> {
    let value = object.get(field).and_then(Value::as_str).ok_or_else(|| {
        StoryFailure::new(
            "AUTHORING_STORY_INPUT_INVALID",
            format!("'{field}' must be a {kind} string"),
        )
    })?;
    if value.is_empty() {
        return Err(StoryFailure::new(
            "AUTHORING_STORY_INPUT_INVALID",
            format!("'{field}' must not be empty"),
        ));
    }
    if value.len() > max_bytes {
        return Err(StoryFailure::new(
            limit_code,
            format!("'{field}' exceeds the {max_bytes}-byte limit"),
        ));
    }
    Ok(value)
}

fn required_profile(object: &Map<String, Value>) -> Result<ValidationProfile, StoryFailure> {
    match object.get("profile").and_then(Value::as_str) {
        Some("production") => Ok(ValidationProfile::Production),
        Some("experimental") => Ok(ValidationProfile::Experimental),
        _ => Err(StoryFailure::new(
            "AUTHORING_STORY_PROFILE_INVALID",
            "'profile' must be 'production' or 'experimental'",
        )),
    }
}

fn map_project_error(error: ProjectDocumentError) -> StoryFailure {
    match error {
        ProjectDocumentError::InputTooLarge { .. } => StoryFailure::new(
            "AUTHORING_STORY_PROJECT_LIMIT",
            format!("authoring project JSON exceeds the {MAX_PROJECT_JSON_BYTES}-byte limit"),
        ),
        error => StoryFailure::new("AUTHORING_STORY_PROJECT_INVALID", error.to_string()),
    }
}

fn map_mutation_error(error: StoryDraftInsertJsonError) -> StoryFailure {
    match error {
        StoryDraftInsertJsonError::InputTooLarge { .. } => StoryFailure::new(
            "AUTHORING_STORY_MUTATION_LIMIT",
            format!(
                "story mutation JSON exceeds the {MAX_STORY_DRAFT_INSERT_JSON_BYTES}-byte limit"
            ),
        ),
        StoryDraftInsertJsonError::InvalidJson(_) => {
            StoryFailure::new("AUTHORING_STORY_MUTATION_INVALID", error.to_string())
        }
    }
}

fn map_transaction_error(error: StoryDraftInsertError) -> StoryFailure {
    StoryFailure::new("AUTHORING_STORY_TRANSACTION_FAILED", error.to_string())
}

fn diagnostics_to_wire(
    diagnostics: Vec<Diagnostic>,
    response_limit: usize,
    mut estimated_bytes: usize,
) -> Result<Vec<Value>, StoryFailure> {
    if diagnostics.len() > MAX_DIAGNOSTICS {
        return Err(response_limit_failure());
    }
    let mut wire = Vec::new();
    for diagnostic in diagnostics {
        if diagnostic
            .property_path
            .as_ref()
            .is_some_and(|path| path.len() > MAX_DIAGNOSTIC_PROPERTY_PATH_BYTES)
            || diagnostic.related_entities.len() > MAX_DIAGNOSTIC_RELATED_ENTITIES
        {
            return Err(response_limit_failure());
        }
        let related_estimate = diagnostic
            .related_entities
            .len()
            .checked_mul(35)
            .ok_or_else(response_limit_failure)?;
        if estimated_bytes
            .checked_add(related_estimate)
            .is_none_or(|bytes| bytes > response_limit)
        {
            return Err(response_limit_failure());
        }
        let message =
            truncate_utf8_with_suffix(diagnostic.message, MAX_DIAGNOSTIC_MESSAGE_BYTES, "...");
        let item = json!({
            "code": diagnostic.code,
            "severity": diagnostic.severity,
            "entity": diagnostic.entity.map(|entity| entity.to_string()),
            "property_path": diagnostic.property_path,
            "message": message,
            "related_entities": diagnostic
                .related_entities
                .into_iter()
                .map(|entity| entity.to_string())
                .collect::<Vec<_>>(),
            "blocks_build": diagnostic.blocks_build,
        });
        let item_bytes = serde_json::to_vec(&item).map_err(|_| {
            StoryFailure::new(
                "AUTHORING_STORY_RESPONSE_SERIALIZE",
                "story diagnostic serialization failed",
            )
        })?;
        estimated_bytes = estimated_bytes
            .checked_add(item_bytes.len() + 1)
            .filter(|bytes| *bytes <= response_limit)
            .ok_or_else(response_limit_failure)?;
        wire.push(item);
    }
    Ok(wire)
}

fn enforce_response_budget(response: Value, limit: usize) -> Result<Value, StoryFailure> {
    match serde_json::to_vec(&response) {
        Ok(encoded) if encoded.len() <= limit => Ok(response),
        Ok(_) => Err(response_limit_failure()),
        Err(_) => Err(StoryFailure::new(
            "AUTHORING_STORY_RESPONSE_SERIALIZE",
            "story transaction response serialization failed",
        )),
    }
}

fn response_limit_failure() -> StoryFailure {
    StoryFailure::new(
        "AUTHORING_STORY_RESPONSE_LIMIT",
        format!(
            "story transaction response exceeds the {MAX_AUTHORING_STORY_RESPONSE_BYTES}-byte limit"
        ),
    )
}

fn truncate_utf8_with_suffix(mut value: String, max_bytes: usize, suffix: &str) -> String {
    if value.len() <= max_bytes {
        return value;
    }
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
    use super::*;
    use crate::execute_json;

    fn project_json_at(schema_revision: u32, revision: u64) -> String {
        let raw = json!({
            "format": 2,
            "schema_revision": schema_revision,
            "project_id": "01010101010101010101010101010101",
            "revision": revision,
            "meta": {"name": "Story transaction", "version": "0.1", "author": "tests"},
            "target": {"executable": {
                "byte_len": 1_000_000,
                "sha256": "0101010101010101010101010101010101010101010101010101010101010101"
            }},
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

    fn project_json(schema_revision: u32) -> String {
        project_json_at(schema_revision, 7)
    }

    fn generation() -> Value {
        json!({"executable": {
            "byte_len": 1_000_000,
            "sha256": "0101010101010101010101010101010101010101010101010101010101010101"
        }})
    }

    fn parent(seal: &str, selector: &str, runtime_class: &str) -> Value {
        json!({
            "generation": generation(),
            "source_seal": {"byte_len": 20_000, "sha256": seal.repeat(32)},
            "catalog_layer": "base-game.g1r.characters",
            "canonical_selector": selector,
            "runtime_class": runtime_class,
        })
    }

    fn mutation_json(expected_revision: u64) -> String {
        json!({
            "expected_project_id": "01010101010101010101010101010101",
            "expected_revision": expected_revision,
            "draft_id": "10101010101010101010101010101010",
            "script_module_id": "11111111111111111111111111111111",
            "display_name": "NPC GoreGateGuard",
            "draft": {
                "kind": "npc",
                "input": {
                    "module_namespace": "GoreMods.Npcs.GateGuard",
                    "unique_name": "GoreGateGuard",
                    "parent_character_definition": parent(
                        "02",
                        "CatalogCharacterDefinition_Asghan",
                        "UCharacterDefinition_Human_OM_GRD_Asghan_263",
                    ),
                    "parent_ai_agent_config": parent(
                        "03",
                        "CatalogAiAgentConfig_Asghan",
                        "UAIAgentConfig_Human_OM_GRD_Asghan_263",
                    ),
                    "parent_spawn_definition": parent(
                        "04",
                        "CatalogSpawnDefinition_Asghan",
                        "USpawnAIAgentDefinition_OM_GRD_Asghan_263",
                    ),
                }
            }
        })
        .to_string()
    }

    fn quest_mutation_json(expected_revision: u64) -> String {
        json!({
            "expected_project_id": "01010101010101010101010101010101",
            "expected_revision": expected_revision,
            "draft_id": "10101010101010101010101010101010",
            "script_module_id": "11111111111111111111111111111111",
            "display_name": "Quest GORE_GATE_TRIAL",
            "draft": {
                "kind": "quest",
                "input": {
                    "module_namespace": "GoreMods.Quests.GateTrial",
                    "technical_id": "GORE_GATE_TRIAL",
                    "text_helper": "GoreGateTrialText",
                    "parent_quest": {
                        "generation": generation(),
                        "source_seal": {
                            "byte_len": 30_000,
                            "sha256": "05".repeat(32),
                        },
                        "catalog_layer": "base-game.g1r.quests",
                        "canonical_selector": "CatalogQuest_AsghanParent",
                        "runtime_class": "UQuest_SwampCamp_SCCHAPTER2",
                    },
                    "giver": {
                        "generation": generation(),
                        "source_seal": {
                            "byte_len": 40_000,
                            "sha256": "06".repeat(32),
                        },
                        "catalog_layer": "base-game.g1r.characters",
                        "canonical_selector": "CatalogCharacter_Asghan",
                        "runtime_unique_name": "OM_GRD_Asghan_263",
                    },
                    "title": "Asghan's Trial",
                    "description": "Prove that the gate is secure.",
                    "objective_title": "Report to Asghan",
                    "collision_catalog": {
                        "generation": generation(),
                        "source_seal": {
                            "byte_len": 50_000,
                            "sha256": "07".repeat(32),
                        },
                        "catalog_layer": "resolved-loadout.scripts.v1",
                        "modules": [],
                        "relative_paths": [],
                        "symbols": [],
                    },
                }
            }
        })
        .to_string()
    }

    fn call_raw(project_json: String, mutation_json: String, profile: &str) -> String {
        execute_json(
            &json!({
                "command": "authoring_project_story_draft_insert_v1",
                "payload": {
                    "project_json": project_json,
                    "mutation_json": mutation_json,
                    "profile": profile,
                }
            })
            .to_string(),
        )
    }

    fn call(project_json: String, mutation_json: String, profile: &str) -> Value {
        serde_json::from_str(&call_raw(project_json, mutation_json, profile)).unwrap()
    }

    #[test]
    fn applied_npc_returns_one_exact_canonical_candidate_and_closed_metadata() {
        let project = project_json(2);
        let mutation = mutation_json(7);
        let expected_binding = story_draft_insert_request_binding_sha256(
            &project,
            &mutation,
            ValidationProfile::Experimental,
        )
        .to_string();
        let response = call(project, mutation, "experimental");
        assert_eq!(response["ok"], true);
        assert_eq!(response["outcome"], "applied");
        assert_eq!(response["request_binding_sha256"], expected_binding);
        assert_eq!(response["revision"], 8);
        assert_eq!(response["draft_id"], "10101010101010101010101010101010");
        assert_eq!(response["draft_kind"], "npc_draft");
        assert_eq!(
            response["script_module_id"],
            "11111111111111111111111111111111"
        );
        assert_eq!(response["blocks_build"], true);
        assert!(response["diagnostics"].as_array().unwrap().iter().any(
            |diagnostic| diagnostic["code"] == "REVISION2_COMBINED_VALIDATION_UNAVAILABLE"
                && diagnostic["blocks_build"] == true
        ));
        let candidate = response["project_json"].as_str().unwrap();
        assert_eq!(
            ProjectDocument::from_json(candidate)
                .unwrap()
                .to_canonical_json()
                .unwrap(),
            candidate
        );
    }

    #[test]
    fn semantic_rejection_never_returns_candidate_or_applied_metadata() {
        let project = project_json(2);
        let mutation = mutation_json(6);
        let expected_binding = story_draft_insert_request_binding_sha256(
            &project,
            &mutation,
            ValidationProfile::Production,
        )
        .to_string();
        let response = call(project, mutation, "production");
        assert_eq!(response["ok"], true);
        assert_eq!(response["outcome"], "rejected");
        assert_eq!(response["request_binding_sha256"], expected_binding);
        assert!(!response["diagnostics"].as_array().unwrap().is_empty());
        for forbidden in [
            "project_json",
            "revision",
            "draft_id",
            "draft_kind",
            "script_module_id",
            "blocks_build",
        ] {
            assert!(response.get(forbidden).is_none(), "unexpected {forbidden}");
        }
    }

    #[test]
    fn invalid_generator_input_is_a_bound_typed_rejection() {
        let project = project_json(2);
        let mutation =
            mutation_json(7).replace("GoreMods.Npcs.GateGuard", "module namespace with spaces");
        let response = call(project.clone(), mutation.clone(), "experimental");
        assert_eq!(response["ok"], true);
        assert_eq!(response["outcome"], "rejected");
        assert_eq!(
            response["request_binding_sha256"],
            story_draft_insert_request_binding_sha256(
                &project,
                &mutation,
                ValidationProfile::Experimental,
            )
            .to_string()
        );
        assert!(
            response["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic["severity"] == "error"
                    && diagnostic["blocks_build"] == true)
        );
        assert!(response.get("project_json").is_none());
    }

    #[test]
    fn quest_insert_requires_trusted_collision_inventory_before_transaction() {
        let project = project_json(2);
        let mutation = quest_mutation_json(7);
        let first = call_raw(project.clone(), mutation.clone(), "experimental");
        let second = call_raw(project.clone(), mutation.clone(), "experimental");
        assert_eq!(first, second);
        let response: Value = serde_json::from_str(&first).unwrap();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_STORY_QUEST_INVENTORY_REQUIRED"
        );
        assert!(response.get("outcome").is_none());
        assert!(response.get("project_json").is_none());
        assert!(response.get("revision").is_none());
    }

    #[test]
    fn signed_revision_wire_boundaries_and_unicode_diagnostics_fail_closed() {
        let maximum_base = MAX_AUTHORING_STORY_BASE_REVISION;
        let boundary = call(
            project_json_at(2, maximum_base),
            mutation_json(maximum_base),
            "experimental",
        );
        assert_eq!(boundary["ok"], true);
        assert_eq!(boundary["outcome"], "applied");
        assert_eq!(boundary["revision"], i64::MAX as u64);

        for limited in [
            call(
                project_json_at(2, i64::MAX as u64),
                mutation_json(i64::MAX as u64),
                "experimental",
            ),
            call(
                project_json(2),
                mutation_json(i64::MAX as u64),
                "experimental",
            ),
            call(
                project_json_at(2, u64::MAX),
                mutation_json(u64::MAX),
                "experimental",
            ),
        ] {
            assert_eq!(
                limited["error"]["code"],
                "AUTHORING_STORY_REVISION_WIRE_LIMIT"
            );
            assert!(limited.get("project_json").is_none());
        }

        let diagnostics = diagnostics_to_wire(
            vec![Diagnostic {
                code: DiagnosticCode::InvalidStoryMutation,
                severity: DiagnosticSeverity::Error,
                entity: None,
                property_path: Some("draft.input".to_owned()),
                message: "🦀".repeat(MAX_DIAGNOSTIC_MESSAGE_BYTES),
                related_entities: Vec::new(),
                blocks_build: true,
            }],
            MAX_AUTHORING_STORY_RESPONSE_BYTES,
            0,
        )
        .unwrap();
        let message = diagnostics[0]["message"].as_str().unwrap();
        assert!(message.len() <= MAX_DIAGNOSTIC_MESSAGE_BYTES);
        assert!(message.ends_with("..."));
        assert!(std::str::from_utf8(message.as_bytes()).is_ok());
    }

    #[test]
    fn fractional_and_overflowing_json_revisions_never_reach_the_transaction() {
        let fractional_project = project_json(2).replacen("\"revision\":7", "\"revision\":7.0", 1);
        assert_eq!(
            call(fractional_project, mutation_json(7), "experimental")["error"]["code"],
            "AUTHORING_STORY_PROJECT_INVALID"
        );

        for invalid_mutation in [
            mutation_json(7).replacen("\"expected_revision\":7", "\"expected_revision\":7.0", 1),
            mutation_json(7).replacen(
                "\"expected_revision\":7",
                "\"expected_revision\":18446744073709551616",
                1,
            ),
        ] {
            assert_eq!(
                call(project_json(2), invalid_mutation, "experimental")["error"]["code"],
                "AUTHORING_STORY_MUTATION_INVALID"
            );
        }
    }

    #[test]
    fn project_must_be_exact_canonical_revision2_and_both_raw_inputs_reject_duplicates() {
        let noncanonical = format!(" {}", project_json(2));
        assert_eq!(
            call(noncanonical, mutation_json(7), "production")["error"]["code"],
            "AUTHORING_STORY_PROJECT_NONCANONICAL"
        );
        assert_eq!(
            call(project_json(1), mutation_json(7), "production")["error"]["code"],
            "AUTHORING_STORY_PROJECT_REVISION_REQUIRED"
        );
        let duplicate_project =
            project_json(2).replacen("\"revision\":7", "\"revision\":7,\"revision\":7", 1);
        assert_eq!(
            call(duplicate_project, mutation_json(7), "production")["error"]["code"],
            "AUTHORING_STORY_PROJECT_INVALID"
        );
        let duplicate_mutation = mutation_json(7).replacen(
            "\"expected_revision\":7",
            "\"expected_revision\":7,\"expected_revision\":7",
            1,
        );
        assert_eq!(
            call(project_json(2), duplicate_mutation, "production")["error"]["code"],
            "AUTHORING_STORY_MUTATION_INVALID"
        );
    }

    #[test]
    fn payload_profiles_input_limits_and_response_budget_fail_closed() {
        let cases = [
            insert_story_draft_v1(Value::Null),
            insert_story_draft_v1(json!({
                "project_json": project_json(2),
                "mutation_json": mutation_json(7),
                "profile": "preview",
            })),
            insert_story_draft_v1(json!({
                "project_json": "x".repeat(MAX_PROJECT_JSON_BYTES + 1),
                "mutation_json": mutation_json(7),
                "profile": "production",
            })),
            insert_story_draft_v1(json!({
                "project_json": project_json(2),
                "mutation_json": "x".repeat(MAX_STORY_DRAFT_INSERT_JSON_BYTES + 1),
                "profile": "production",
            })),
        ];
        assert_eq!(cases[0]["error"]["code"], "AUTHORING_STORY_PAYLOAD_INVALID");
        assert_eq!(cases[1]["error"]["code"], "AUTHORING_STORY_PROFILE_INVALID");
        assert_eq!(cases[2]["error"]["code"], "AUTHORING_STORY_PROJECT_LIMIT");
        assert_eq!(cases[3]["error"]["code"], "AUTHORING_STORY_MUTATION_LIMIT");

        let limited = command_response(insert_story_draft_v1_inner(
            &json!({
                "project_json": project_json(2),
                "mutation_json": mutation_json(7),
                "profile": "production",
            }),
            128,
        ));
        assert_eq!(limited["error"]["code"], "AUTHORING_STORY_RESPONSE_LIMIT");
    }
}
