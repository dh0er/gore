use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::{
    AssetStoreIndex, ContentSeal, DiagnosticCode, DiagnosticSeverity, EntityId, FormatV2,
    GameGenerationAnchor, NpcDraftCreateInput, ProjectDocument, ProjectId, ProjectMeta,
    ProjectRevision2, ProjectV2, QuestCollisionCatalogInput, QuestDraftCreateInput,
    Revision2EntityKind as EntityKind, Revision2EntityPayload as EntityPayload,
    Revision2NpcParentClassInput as NpcParentClassInput,
    Revision2QuestGiverInput as QuestGiverInput, Revision2QuestParentInput as QuestParentInput,
    SchemaRevisionV1, SchemaRevisionV2, Sha256Digest, StoryDraftCreate, StoryDraftInsertEvaluation,
    StoryDraftInsertOutcome, StoryDraftInsertRequest, ValidationProfile,
};
use gore_story_build::{
    plan_story_build, StoryBuildError, StoryBuildPlan, StoryBuildPublicationStatus,
    StoryPropertyProvenance, MAX_STORY_BUILD_PLAN_JSON_BYTES, MAX_STORY_BUILD_PROJECT_JSON_BYTES,
    MAX_STORY_BUILD_PROPERTY_PATH_BYTES, MAX_STORY_BUILD_RELATED_ENTITIES_PER_DIAGNOSTIC,
    MAX_STORY_BUILD_SEALED_INPUTS_PER_MODULE,
};
use sha2::{Digest as _, Sha256};

fn project_id(value: u8) -> ProjectId {
    ProjectId::from_bytes([value; 16])
}

fn entity_id(value: u8) -> EntityId {
    EntityId::from_bytes([value; 16])
}

fn seal(value: u8, byte_len: u64) -> ContentSeal {
    ContentSeal {
        byte_len,
        sha256: Sha256Digest::from_bytes([value; 32]),
    }
}

fn generation(value: u8) -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: seal(value, 1_000_000),
    }
}

fn empty_project() -> ProjectRevision2 {
    ProjectRevision2 {
        format: FormatV2,
        schema_revision: SchemaRevisionV2,
        project_id: project_id(1),
        revision: 7,
        meta: ProjectMeta {
            name: "Story build plan".into(),
            version: "0.1".into(),
            author: "test".into(),
        },
        target: generation(1),
        authoring_locales: BTreeSet::new(),
        entities: BTreeMap::new(),
        asset_store: AssetStoreIndex::default(),
    }
}

fn npc_parent(
    target: &GameGenerationAnchor,
    seal_value: u8,
    selector: &str,
    runtime_class: &str,
) -> NpcParentClassInput {
    NpcParentClassInput {
        generation: target.clone(),
        source_seal: seal(seal_value, 20_000),
        catalog_layer: "base-game.g1r.characters".into(),
        canonical_selector: selector.into(),
        runtime_class: runtime_class.into(),
    }
}

fn npc_request(project: &ProjectRevision2) -> StoryDraftInsertRequest {
    StoryDraftInsertRequest {
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        draft_id: entity_id(10),
        script_module_id: entity_id(11),
        display_name: "NPC Gate Guard".into(),
        draft: StoryDraftCreate::Npc(NpcDraftCreateInput {
            module_namespace: "GoreMods.Npcs.GateGuard".into(),
            unique_name: "GoreGateGuard".into(),
            parent_character_definition: npc_parent(
                &project.target,
                2,
                "CatalogCharacterDefinition_Asghan",
                "UCharacterDefinition_Human_OM_GRD_Asghan_263",
            ),
            parent_ai_agent_config: npc_parent(
                &project.target,
                3,
                "CatalogAiAgentConfig_Asghan",
                "UAIAgentConfig_Human_OM_GRD_Asghan_263",
            ),
            parent_spawn_definition: npc_parent(
                &project.target,
                4,
                "CatalogSpawnDefinition_Asghan",
                "USpawnAIAgentDefinition_OM_GRD_Asghan_263",
            ),
        }),
    }
}

fn quest_request(project: &ProjectRevision2) -> StoryDraftInsertRequest {
    StoryDraftInsertRequest {
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        draft_id: entity_id(20),
        script_module_id: entity_id(21),
        display_name: "Quest Asghan Trial".into(),
        draft: StoryDraftCreate::Quest(QuestDraftCreateInput {
            module_namespace: "GoreMods.Quests.AsghanTrial".into(),
            technical_id: "GORE_ASGHAN_TRIAL".into(),
            text_helper: "GoreQuestText".into(),
            parent_quest: QuestParentInput {
                generation: project.target.clone(),
                source_seal: seal(5, 30_000),
                catalog_layer: "base-game.g1r.quests".into(),
                canonical_selector: "CatalogQuest_AsghanParent".into(),
                runtime_class: "UQuest_SwampCamp_SCCHAPTER2".into(),
            },
            giver: QuestGiverInput {
                generation: project.target.clone(),
                source_seal: seal(6, 40_000),
                catalog_layer: "base-game.g1r.characters".into(),
                canonical_selector: "CatalogCharacter_Asghan".into(),
                runtime_unique_name: "OM_GRD_Asghan_263".into(),
            },
            title: "Asghan's Trial".into(),
            description: "Prove that the gate is secure.".into(),
            objective_title: "Report to Asghan".into(),
            collision_catalog: QuestCollisionCatalogInput {
                generation: project.target.clone(),
                source_seal: seal(7, 50_000),
                catalog_layer: "resolved-loadout.scripts.v1".into(),
                modules: BTreeSet::new(),
                relative_paths: BTreeSet::new(),
                symbols: BTreeSet::new(),
            },
        }),
    }
}

fn applied(evaluation: StoryDraftInsertEvaluation) -> StoryDraftInsertOutcome {
    match evaluation {
        StoryDraftInsertEvaluation::Applied(outcome) => *outcome,
        StoryDraftInsertEvaluation::Rejected(rejection) => {
            panic!(
                "unexpected story insert rejection: {:?}",
                rejection.diagnostics
            )
        }
    }
}

fn story_project_json() -> String {
    let project = empty_project();
    let request = npc_request(&project);
    let project = applied(
        project
            .insert_story_draft(request, ValidationProfile::Experimental)
            .unwrap(),
    )
    .project;
    let request = quest_request(&project);
    applied(
        project
            .insert_story_draft(request, ValidationProfile::Experimental)
            .unwrap(),
    )
    .canonical_project_json
}

#[test]
fn canonical_plan_is_deterministic_sealed_ordered_and_reopenable() {
    let project_json = story_project_json();
    let first = plan_story_build(&project_json, ValidationProfile::Experimental).unwrap();
    let second = plan_story_build(&project_json, ValidationProfile::Experimental).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.format(), "story_build_plan");
    assert_eq!(first.schema_revision(), 1);
    assert_eq!(
        first.publication_status,
        StoryBuildPublicationStatus::NotSupported
    );
    assert!(first.blocks_build);
    assert_eq!(first.modules.len(), 2);
    assert!(
        first.modules[0].generated.module_relative_path
            < first.modules[1].generated.module_relative_path
    );
    assert_eq!(first.modules[0].sealed_inputs.len(), 7);
    assert_eq!(first.modules[1].sealed_inputs.len(), 7);

    for module in &first.modules {
        assert_eq!(
            module.generated.status,
            gore_authoring::ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED
        );
        assert_eq!(
            module.persisted_source.content.byte_len,
            module.generated.source.len() as u64
        );
        assert_eq!(
            module.persisted_source.content.sha256,
            Sha256Digest::from_bytes(Sha256::digest(module.generated.source.as_bytes()).into())
        );
        assert!(matches!(
            &module.draft_input.provenance,
            StoryPropertyProvenance::Entity { property_path, .. }
                if property_path == "payload.data.input"
        ));
        assert!(matches!(
            &module.persisted_source.provenance,
            StoryPropertyProvenance::Entity {
                entity_id,
                entity_kind: EntityKind::ScriptModule,
                property_path,
                ..
            } if *entity_id == module.script_module.id && property_path == "payload.data.source"
        ));
    }

    let json = first.to_canonical_json().unwrap();
    assert!(json.len() <= MAX_STORY_BUILD_PLAN_JSON_BYTES);
    assert_eq!(StoryBuildPlan::from_json(&json).unwrap(), first);
    first.verify_against_project_json(&project_json).unwrap();
    assert_eq!(
        first.content_seal().unwrap(),
        second.content_seal().unwrap()
    );
    assert_eq!(
        first.project.canonical_document.sha256,
        Sha256Digest::from_bytes(Sha256::digest(project_json.as_bytes()).into())
    );
}

#[test]
fn experimental_profile_cannot_unblock_runtime_unqualified_story_sources() {
    let plan = plan_story_build(&story_project_json(), ValidationProfile::Experimental).unwrap();
    let runtime_blockers: Vec<_> = plan
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == DiagnosticCode::RuntimeUnqualified)
        .collect();

    assert_eq!(runtime_blockers.len(), 2);
    assert!(runtime_blockers.iter().all(|diagnostic| {
        diagnostic.severity == DiagnosticSeverity::Error && diagnostic.blocks_build
    }));
    assert!(plan.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::Revision2CombinedValidationUnavailable
            && diagnostic.severity == DiagnosticSeverity::Error
            && diagnostic.blocks_build
    }));
}

#[test]
fn project_boundary_uses_strict_core_dispatch_and_exact_canonical_spelling() {
    let canonical = story_project_json();
    let with_whitespace = format!(" {canonical}");
    assert!(matches!(
        plan_story_build(&with_whitespace, ValidationProfile::Production),
        Err(StoryBuildError::NonCanonicalProjectJson)
    ));

    let revision_marker = "\"revision\":9";
    assert!(canonical.contains(revision_marker));
    let duplicate = canonical.replacen(revision_marker, "\"revision\":9,\"revision\":9", 1);
    assert!(matches!(
        plan_story_build(&duplicate, ValidationProfile::Production),
        Err(StoryBuildError::InvalidProjectDocument(_))
    ));

    let oversized = " ".repeat(MAX_STORY_BUILD_PROJECT_JSON_BYTES + 1);
    assert!(matches!(
        plan_story_build(&oversized, ValidationProfile::Production),
        Err(StoryBuildError::ProjectJsonTooLarge { .. })
    ));
}

#[test]
fn revision_one_is_never_implicitly_migrated() {
    let revision_one = ProjectV2 {
        format: FormatV2,
        schema_revision: SchemaRevisionV1,
        project_id: project_id(8),
        revision: 1,
        meta: ProjectMeta {
            name: "Revision one".into(),
            version: String::new(),
            author: String::new(),
        },
        target: generation(8),
        authoring_locales: BTreeSet::new(),
        entities: BTreeMap::new(),
        asset_store: AssetStoreIndex::default(),
    };
    let json = revision_one.to_canonical_json().unwrap();

    assert!(matches!(
        plan_story_build(&json, ValidationProfile::Production),
        Err(StoryBuildError::Revision2Required)
    ));
}

#[test]
fn plan_reopen_rejects_duplicate_noncanonical_and_tampered_json() {
    let plan = plan_story_build(&story_project_json(), ValidationProfile::Production).unwrap();
    let canonical = plan.to_canonical_json().unwrap();

    let duplicate = canonical.replacen(
        "\"blocks_build\":true",
        "\"blocks_build\":true,\"blocks_build\":true",
        1,
    );
    assert!(matches!(
        StoryBuildPlan::from_json(&duplicate),
        Err(StoryBuildError::InvalidPlanJson(_))
    ));

    let value: serde_json::Value = serde_json::from_str(&canonical).unwrap();
    let pretty = serde_json::to_string_pretty(&value).unwrap();
    assert!(matches!(
        StoryBuildPlan::from_json(&pretty),
        Err(StoryBuildError::NonCanonicalPlanJson)
    ));

    let mut tampered: serde_json::Value = serde_json::from_str(&canonical).unwrap();
    tampered["modules"][0]["generated"]["source"] = serde_json::Value::String("tampered".into());
    let tampered = serde_json::to_string(&tampered).unwrap();
    assert!(matches!(
        StoryBuildPlan::from_json(&tampered),
        Err(StoryBuildError::Invariant(_))
    ));

    let mut oversized_path: serde_json::Value = serde_json::from_str(&canonical).unwrap();
    oversized_path["modules"][0]["draft_input"]["provenance"]["property_path"] =
        serde_json::Value::String("p".repeat(MAX_STORY_BUILD_PROPERTY_PATH_BYTES + 1));
    assert!(matches!(
        StoryBuildPlan::from_json(&serde_json::to_string(&oversized_path).unwrap()),
        Err(StoryBuildError::InvalidPlanJson(_))
    ));

    let mut too_many_seals: serde_json::Value = serde_json::from_str(&canonical).unwrap();
    let first_seal = too_many_seals["modules"][0]["sealed_inputs"][0].clone();
    let seals = too_many_seals["modules"][0]["sealed_inputs"]
        .as_array_mut()
        .unwrap();
    while seals.len() <= MAX_STORY_BUILD_SEALED_INPUTS_PER_MODULE {
        seals.push(first_seal.clone());
    }
    assert!(matches!(
        StoryBuildPlan::from_json(&serde_json::to_string(&too_many_seals).unwrap()),
        Err(StoryBuildError::InvalidPlanJson(_))
    ));

    let mut too_many_related: serde_json::Value = serde_json::from_str(&canonical).unwrap();
    too_many_related["diagnostics"][0]["related_entities"] = serde_json::Value::Array(
        (0..=MAX_STORY_BUILD_RELATED_ENTITIES_PER_DIAGNOSTIC)
            .map(|_| serde_json::Value::String(entity_id(90).to_string()))
            .collect(),
    );
    assert!(matches!(
        StoryBuildPlan::from_json(&serde_json::to_string(&too_many_related).unwrap()),
        Err(StoryBuildError::InvalidPlanJson(_))
    ));

    let mut wrong_module_id = plan.clone();
    wrong_module_id.modules[0].script_module.id = entity_id(99);
    assert!(matches!(
        wrong_module_id.to_canonical_json(),
        Err(StoryBuildError::Invariant(_))
    ));
    assert!(matches!(
        wrong_module_id.verify_against_project_json(&story_project_json()),
        Err(StoryBuildError::ProjectBindingMismatch)
    ));
}

#[test]
fn drifted_persisted_source_is_excluded_and_remains_explicitly_blocked() {
    let document = ProjectDocument::from_json(&story_project_json()).unwrap();
    let ProjectDocument::Revision2(mut project) = document else {
        panic!("expected revision 2")
    };
    let module_id = entity_id(11);
    let EntityPayload::ScriptModule(module) =
        &mut project.entities.get_mut(&module_id).unwrap().payload
    else {
        panic!("expected ScriptModule")
    };
    module.source.push_str("// drift");
    let drifted_json = project.to_canonical_json().unwrap();

    let plan = plan_story_build(&drifted_json, ValidationProfile::Experimental).unwrap();
    assert_eq!(plan.modules.len(), 1);
    assert!(plan.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::GeneratedScriptDrift
            && diagnostic.entity == Some(module_id)
            && diagnostic.blocks_build
    }));
    assert!(plan.blocks_build);
}

#[test]
fn semantically_invalid_project_returns_blocking_diagnostics_instead_of_disappearing() {
    let mut project = empty_project();
    project.target.executable.byte_len = 0;
    let json = project.to_canonical_json().unwrap();

    let plan = plan_story_build(&json, ValidationProfile::Experimental).unwrap();
    assert!(plan.modules.is_empty());
    assert!(plan.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::InvalidGenerationAnchor && diagnostic.blocks_build
    }));
    assert!(plan.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::Revision2CombinedValidationUnavailable
            && diagnostic.blocks_build
    }));
}
