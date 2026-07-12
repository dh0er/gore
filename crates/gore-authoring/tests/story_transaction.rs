use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::{
    story_draft_insert_request_binding_sha256, AssetMeta, AssetStoreIndex, ContentSeal,
    DiagnosticCode, EntityId, FormatV2, GameGenerationAnchor, NpcDraftCreateInput, ProjectDocument,
    ProjectId, ProjectMeta, ProjectRevision2, ProjectV2, QuestCollisionCatalogInput,
    QuestDraftCreateInput, Revision2Entity as Entity, Revision2EntityKind as EntityKind,
    Revision2EntityPayload as EntityPayload, Revision2LocalizationEntry as LocalizationEntry,
    Revision2NpcParentClassInput as NpcParentClassInput, Revision2OriginRef as OriginRef,
    Revision2QuestGiverInput as QuestGiverInput, Revision2QuestParentInput as QuestParentInput,
    SchemaRevisionV1, SchemaRevisionV2, StoryDraftCreate, StoryDraftInsertEvaluation,
    StoryDraftInsertOutcome, StoryDraftInsertRequest, ValidationProfile, WorkingStoreLimits,
    MAX_STORY_DRAFT_INSERT_JSON_BYTES,
};

#[test]
fn request_binding_has_one_exact_domain_separated_little_endian_spelling() {
    let production = story_draft_insert_request_binding_sha256(
        "project",
        "mutation",
        ValidationProfile::Production,
    );
    assert_eq!(
        production.to_string(),
        "7141d4e86bcf237fda2adbfd5506b585500d9cf36a0bd0bec42377721ca8a95d"
    );
    assert_eq!(
        production,
        story_draft_insert_request_binding_sha256(
            "project",
            "mutation",
            ValidationProfile::Production,
        )
    );
    assert_ne!(
        production,
        story_draft_insert_request_binding_sha256(
            "project",
            "mutation",
            ValidationProfile::Experimental,
        )
    );
}

fn project_id(value: u8) -> ProjectId {
    ProjectId::from_bytes([value; 16])
}

fn entity_id(value: u8) -> EntityId {
    EntityId::from_bytes([value; 16])
}

fn seal(value: u8, byte_len: u64) -> ContentSeal {
    ContentSeal {
        byte_len,
        sha256: gore_authoring::Sha256Digest::from_bytes([value; 32]),
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
            name: "Story transaction".into(),
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

fn npc_request(
    project: &ProjectRevision2,
    draft: u8,
    module: u8,
    namespace: &str,
    unique_name: &str,
) -> StoryDraftInsertRequest {
    StoryDraftInsertRequest {
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        draft_id: entity_id(draft),
        script_module_id: entity_id(module),
        display_name: format!("NPC {unique_name}"),
        draft: StoryDraftCreate::Npc(NpcDraftCreateInput {
            module_namespace: namespace.into(),
            unique_name: unique_name.into(),
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

fn quest_request(
    project: &ProjectRevision2,
    draft: u8,
    module: u8,
    namespace: &str,
    technical_id: &str,
) -> StoryDraftInsertRequest {
    StoryDraftInsertRequest {
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        draft_id: entity_id(draft),
        script_module_id: entity_id(module),
        display_name: format!("Quest {technical_id}"),
        draft: StoryDraftCreate::Quest(QuestDraftCreateInput {
            module_namespace: namespace.into(),
            technical_id: technical_id.into(),
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
            panic!("unexpected rejection: {:?}", rejection.diagnostics)
        }
    }
}

fn rejection_codes(evaluation: StoryDraftInsertEvaluation) -> BTreeSet<DiagnosticCode> {
    match evaluation {
        StoryDraftInsertEvaluation::Rejected(rejection) => rejection
            .diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect(),
        StoryDraftInsertEvaluation::Applied(_) => panic!("unexpected applied transaction"),
    }
}

#[test]
fn npc_insert_publishes_exact_owned_pair_and_canonical_reopen() {
    let project = empty_project();
    let request = npc_request(&project, 10, 11, "GoreMods.Npcs.GateGuard", "GoreGateGuard");
    let outcome = applied(
        project
            .insert_story_draft(request, ValidationProfile::Experimental)
            .unwrap(),
    );

    assert_eq!(outcome.project.revision, 8);
    assert_eq!(outcome.project.entities.len(), 2);
    assert_eq!(outcome.draft_id, entity_id(10));
    assert_eq!(outcome.draft_kind, EntityKind::NpcDraft);
    assert_eq!(outcome.script_module_id, entity_id(11));
    assert!(outcome.blocks_build);
    assert!(outcome.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::Revision2CombinedValidationUnavailable
            && diagnostic.blocks_build
    }));

    let draft_entity = &outcome.project.entities[&entity_id(10)];
    assert_eq!(
        draft_entity.origin,
        OriginRef::New {
            authored_runtime_id: "GoreGateGuard".into()
        }
    );
    let EntityPayload::NpcDraft(draft) = &draft_entity.payload else {
        panic!("expected NPC Draft")
    };
    assert_eq!(draft.script_module.id, entity_id(11));
    let module_entity = &outcome.project.entities[&entity_id(11)];
    let EntityPayload::ScriptModule(module) = &module_entity.payload else {
        panic!("expected generated ScriptModule")
    };
    assert_eq!(module.owner.id, entity_id(10));
    assert_eq!(
        draft
            .regenerate_script_module(module.owner.clone())
            .unwrap(),
        *module
    );
    assert_eq!(
        ProjectRevision2::from_json(&outcome.canonical_project_json).unwrap(),
        outcome.project
    );
}

#[test]
fn quest_insert_uses_stable_draft_id_as_quest_id() {
    let project = empty_project();
    let request = quest_request(
        &project,
        20,
        21,
        "GoreMods.Quests.AsghanTrial",
        "GORE_ASGHAN_TRIAL",
    );
    let outcome = applied(
        project
            .insert_story_draft(request, ValidationProfile::Production)
            .unwrap(),
    );

    let draft_entity = &outcome.project.entities[&entity_id(20)];
    let EntityPayload::QuestDraft(draft) = &draft_entity.payload else {
        panic!("expected Quest Draft")
    };
    assert_eq!(draft.input.quest_id, entity_id(20));
    assert_eq!(draft.input.target, outcome.project.target);
    assert_eq!(draft.script_module.id, entity_id(21));
    assert_eq!(outcome.project.revision, 8);
    assert!(outcome.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::RuntimeUnqualified && diagnostic.blocks_build
    }));
}

#[test]
fn identical_base_and_request_produce_byte_identical_project() {
    let project = empty_project();
    let request = npc_request(&project, 10, 11, "GoreMods.Npcs.GateGuard", "GoreGateGuard");
    let first = applied(
        project
            .clone()
            .insert_story_draft(request.clone(), ValidationProfile::Experimental)
            .unwrap(),
    );
    let second = applied(
        project
            .insert_story_draft(request, ValidationProfile::Experimental)
            .unwrap(),
    );
    assert_eq!(first.canonical_project_json, second.canonical_project_json);
    assert_eq!(first.draft_id, second.draft_id);
    assert_eq!(first.script_module_id, second.script_module_id);
}

#[test]
fn a_second_non_colliding_story_insert_succeeds_and_increments_once() {
    let project = empty_project();
    let first = applied(
        project
            .clone()
            .insert_story_draft(
                npc_request(&project, 10, 11, "GoreMods.Npcs.GateGuard", "GoreGateGuard"),
                ValidationProfile::Experimental,
            )
            .unwrap(),
    );
    let second_request = quest_request(
        &first.project,
        20,
        21,
        "GoreMods.Quests.AsghanTrial",
        "GORE_ASGHAN_TRIAL",
    );
    let second = applied(
        first
            .project
            .insert_story_draft(second_request, ValidationProfile::Experimental)
            .unwrap(),
    );

    assert_eq!(second.project.revision, 9);
    assert_eq!(second.project.entities.len(), 4);
    assert_eq!(second.draft_kind, EntityKind::QuestDraft);
    assert!(second.project.entities.contains_key(&entity_id(10)));
    assert!(second.project.entities.contains_key(&entity_id(20)));
}

#[test]
fn project_identity_revision_and_id_conflicts_reject_without_candidate() {
    let project = empty_project();
    let original = project.to_canonical_json().unwrap();
    let mut request = npc_request(&project, 10, 10, "GoreMods.Npcs.Guard", "GoreGuard");
    request.expected_project_id = project_id(9);
    request.expected_revision = 99;
    request.draft_id = EntityId::from_bytes([0; 16]);
    request.script_module_id = EntityId::from_bytes([0; 16]);

    let codes = rejection_codes(
        project
            .clone()
            .insert_story_draft(request, ValidationProfile::Experimental)
            .unwrap(),
    );
    assert!(codes.contains(&DiagnosticCode::ProjectIdentityMismatch));
    assert!(codes.contains(&DiagnosticCode::ProjectRevisionConflict));
    assert!(codes.contains(&DiagnosticCode::InvalidStoryMutation));
    assert!(codes.contains(&DiagnosticCode::DuplicateEntityId));
    assert_eq!(project.to_canonical_json().unwrap(), original);
}

#[test]
fn existing_entity_ids_and_revision_overflow_reject() {
    let project = empty_project();
    let first_request = npc_request(
        &project,
        10,
        11,
        "GoreMods.Npcs.FirstGuard",
        "GoreFirstGuard",
    );
    let first = applied(
        project
            .insert_story_draft(first_request, ValidationProfile::Experimental)
            .unwrap(),
    );
    let duplicate = npc_request(
        &first.project,
        10,
        12,
        "GoreMods.Npcs.SecondGuard",
        "GoreSecondGuard",
    );
    let codes = rejection_codes(
        first
            .project
            .clone()
            .insert_story_draft(duplicate, ValidationProfile::Experimental)
            .unwrap(),
    );
    assert!(codes.contains(&DiagnosticCode::DuplicateEntityId));

    let mut overflow = empty_project();
    overflow.revision = u64::MAX;
    let request = npc_request(&overflow, 30, 31, "GoreMods.Npcs.Overflow", "GoreOverflow");
    let codes = rejection_codes(
        overflow
            .insert_story_draft(request, ValidationProfile::Experimental)
            .unwrap(),
    );
    assert!(codes.contains(&DiagnosticCode::ProjectRevisionOverflow));
}

#[test]
fn invalid_display_and_catalog_provenance_reject_with_field_diagnostics() {
    let project = empty_project();
    let mut request = npc_request(&project, 10, 11, "GoreMods.Npcs.Guard", "GoreGuard");
    request.display_name = "\n".into();
    let StoryDraftCreate::Npc(input) = &mut request.draft else {
        unreachable!()
    };
    input.parent_character_definition.generation = generation(9);

    let evaluation = project
        .insert_story_draft(request, ValidationProfile::Experimental)
        .unwrap();
    let StoryDraftInsertEvaluation::Rejected(rejection) = evaluation else {
        panic!("expected rejection")
    };
    // Preflight display rejection is fail-fast; no generator candidate is built.
    assert!(rejection.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::InvalidStoryMutation
            && diagnostic.property_path.as_deref() == Some("display_name")
    }));

    let project = empty_project();
    let mut request = npc_request(&project, 10, 11, "GoreMods.Npcs.Guard", "GoreGuard");
    let StoryDraftCreate::Npc(input) = &mut request.draft else {
        unreachable!()
    };
    input.parent_character_definition.generation = generation(9);
    let evaluation = project
        .insert_story_draft(request, ValidationProfile::Experimental)
        .unwrap();
    let StoryDraftInsertEvaluation::Rejected(rejection) = evaluation else {
        panic!("expected rejection")
    };
    assert!(rejection.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::InvalidGeneratorInput
            && diagnostic.property_path.as_deref() == Some("draft.input")
    }));
}

#[test]
fn project_global_namespace_path_runtime_and_symbol_aliases_reject() {
    let project = empty_project();
    let first = applied(
        project
            .insert_story_draft(
                npc_request(
                    &empty_project(),
                    10,
                    11,
                    "GoreMods.Npcs.GateGuard",
                    "GoreGateGuard",
                ),
                ValidationProfile::Experimental,
            )
            .unwrap(),
    );

    let namespace_alias = npc_request(
        &first.project,
        12,
        13,
        "goremods.npcs.gateguard",
        "GoreOtherGuard",
    );
    let codes = rejection_codes(
        first
            .project
            .clone()
            .insert_story_draft(namespace_alias, ValidationProfile::Experimental)
            .unwrap(),
    );
    assert!(codes.contains(&DiagnosticCode::DuplicateScriptModuleNamespace));
    assert!(codes.contains(&DiagnosticCode::DuplicateScriptModulePath));

    let runtime_alias = npc_request(
        &first.project,
        14,
        15,
        "GoreMods.Npcs.OtherGuard",
        "goregateguard",
    );
    let codes = rejection_codes(
        first
            .project
            .clone()
            .insert_story_draft(runtime_alias, ValidationProfile::Experimental)
            .unwrap(),
    );
    assert!(codes.contains(&DiagnosticCode::DuplicateAuthoredRuntimeId));
    assert!(codes.contains(&DiagnosticCode::DuplicateGeneratedSymbol));

    let mut symbol_alias = quest_request(
        &first.project,
        20,
        21,
        "GoreMods.Quests.Trial",
        "GORE_TRIAL",
    );
    let StoryDraftCreate::Quest(input) = &mut symbol_alias.draft else {
        unreachable!()
    };
    input.text_helper = "ucharacterdefinition_human_goregateguard".into();
    let codes = rejection_codes(
        first
            .project
            .insert_story_draft(symbol_alias, ValidationProfile::Experimental)
            .unwrap(),
    );
    assert!(codes.contains(&DiagnosticCode::DuplicateGeneratedSymbol));
}

#[test]
fn runtime_id_collisions_are_story_scoped_but_story_aliases_reject() {
    let mut project = empty_project();
    project.entities.insert(
        entity_id(5),
        Entity {
            id: entity_id(5),
            display_name: "Non-story identity".into(),
            origin: OriginRef::New {
                authored_runtime_id: "GoreGateGuard".into(),
            },
            revision: 0,
            payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                loc_id: "NON_STORY_IDENTITY".into(),
                texts: BTreeMap::new(),
            }),
        },
    );
    let npc = applied(
        project
            .clone()
            .insert_story_draft(
                npc_request(&project, 10, 11, "GoreMods.Npcs.GateGuard", "goregateguard"),
                ValidationProfile::Experimental,
            )
            .unwrap(),
    );
    assert_eq!(npc.project.entities.len(), 3);

    let quest_alias = quest_request(
        &npc.project,
        20,
        21,
        "GoreMods.Quests.GateGuard",
        "GOREGATEGUARD",
    );
    let codes = rejection_codes(
        npc.project
            .insert_story_draft(quest_alias, ValidationProfile::Experimental)
            .unwrap(),
    );
    assert!(codes.contains(&DiagnosticCode::DuplicateAuthoredRuntimeId));
}

#[test]
fn oversized_programmatic_base_entity_and_asset_reject_before_mutation() {
    let limits = WorkingStoreLimits::default();

    let mut oversized_entity = empty_project();
    oversized_entity.entities.insert(
        entity_id(5),
        Entity {
            id: entity_id(5),
            display_name: "x".repeat(limits.max_entity_bytes + 1),
            origin: OriginRef::New {
                authored_runtime_id: "oversized-non-story".into(),
            },
            revision: 0,
            payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                loc_id: "OVERSIZED".into(),
                texts: BTreeMap::new(),
            }),
        },
    );
    let original = oversized_entity.to_canonical_json().unwrap();
    let request = npc_request(
        &oversized_entity,
        10,
        11,
        "GoreMods.Npcs.Guard",
        "GoreGuard",
    );
    let evaluation = oversized_entity
        .clone()
        .insert_story_draft(request, ValidationProfile::Experimental)
        .unwrap();
    let StoryDraftInsertEvaluation::Rejected(rejection) = evaluation else {
        panic!("expected oversized entity rejection")
    };
    assert!(rejection.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::InvalidStoryMutation
            && diagnostic.message.contains("entity bytes")
    }));
    assert_eq!(oversized_entity.to_canonical_json().unwrap(), original);

    let mut oversized_asset = empty_project();
    oversized_asset.asset_store.assets.insert(
        gore_authoring::Sha256Digest::from_bytes([9; 32]),
        AssetMeta {
            byte_len: limits.max_referenced_asset_bytes + 1,
            media_type: "application/octet-stream".into(),
        },
    );
    let original = oversized_asset.to_canonical_json().unwrap();
    let request = npc_request(&oversized_asset, 10, 11, "GoreMods.Npcs.Guard", "GoreGuard");
    let evaluation = oversized_asset
        .clone()
        .insert_story_draft(request, ValidationProfile::Experimental)
        .unwrap();
    let StoryDraftInsertEvaluation::Rejected(rejection) = evaluation else {
        panic!("expected oversized asset rejection")
    };
    assert!(rejection.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::InvalidStoryMutation
            && diagnostic
                .message
                .contains("aggregate referenced asset bytes")
    }));
    assert_eq!(oversized_asset.to_canonical_json().unwrap(), original);
}

#[test]
fn oversized_generated_candidate_rejects_without_returning_a_project() {
    let project = empty_project();
    let mut request = quest_request(
        &project,
        20,
        21,
        "GoreMods.Quests.LargeInventory",
        "GORE_LARGE_INVENTORY",
    );
    let StoryDraftCreate::Quest(input) = &mut request.draft else {
        unreachable!()
    };
    for index in 0..6_000 {
        input
            .collision_catalog
            .symbols
            .insert(format!("existing_symbol_{index:05}_{}", "x".repeat(180)));
    }

    let original = project.to_canonical_json().unwrap();
    let evaluation = project
        .clone()
        .insert_story_draft(request, ValidationProfile::Experimental)
        .unwrap();
    let StoryDraftInsertEvaluation::Rejected(rejection) = evaluation else {
        panic!("expected oversized candidate rejection")
    };
    assert!(rejection.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::InvalidStoryMutation
            && diagnostic.message.contains("entity bytes")
    }));
    assert_eq!(project.to_canonical_json().unwrap(), original);
}

#[test]
fn malformed_base_story_graph_is_not_reinterpreted_by_insert() {
    let project = empty_project();
    let first = applied(
        project
            .clone()
            .insert_story_draft(
                npc_request(&project, 10, 11, "GoreMods.Npcs.GateGuard", "GoreGateGuard"),
                ValidationProfile::Experimental,
            )
            .unwrap(),
    );
    let mut malformed = first.project;
    let EntityPayload::ScriptModule(module) =
        &mut malformed.entities.get_mut(&entity_id(11)).unwrap().payload
    else {
        unreachable!()
    };
    module.owner.id = entity_id(99);
    let request = npc_request(&malformed, 20, 21, "GoreMods.Npcs.Other", "GoreOther");
    let codes = rejection_codes(
        malformed
            .insert_story_draft(request, ValidationProfile::Experimental)
            .unwrap(),
    );
    assert!(codes.contains(&DiagnosticCode::MissingReference));
}

#[test]
fn raw_request_parser_rejects_duplicates_unknown_fields_and_oversize() {
    let project = empty_project();
    let request = npc_request(&project, 10, 11, "GoreMods.Npcs.Guard", "GoreGuard");
    let raw = serde_json::to_string(&request).unwrap();
    assert_eq!(StoryDraftInsertRequest::from_json(&raw).unwrap(), request);

    let duplicate = raw.replacen(
        "\"expected_revision\":7",
        "\"expected_revision\":7,\"expected_revision\":7",
        1,
    );
    assert!(StoryDraftInsertRequest::from_json(&duplicate)
        .unwrap_err()
        .to_string()
        .contains("duplicate JSON object key"));

    let nested_duplicate = raw.replacen(
        "\"unique_name\":\"GoreGuard\"",
        "\"unique_name\":\"GoreGuard\",\"unique_name\":\"GoreGuard\"",
        1,
    );
    assert!(StoryDraftInsertRequest::from_json(&nested_duplicate).is_err());

    let unknown = raw.replacen("\"display_name\":", "\"unknown\":true,\"display_name\":", 1);
    assert!(StoryDraftInsertRequest::from_json(&unknown).is_err());

    let oversized = " ".repeat(MAX_STORY_DRAFT_INSERT_JSON_BYTES + 1);
    assert!(StoryDraftInsertRequest::from_json(&oversized)
        .unwrap_err()
        .to_string()
        .contains("exceeds"));
}

#[test]
fn raw_request_parser_fails_closed_at_excessive_recursion_depth() {
    let nested = format!("{}null{}", "[".repeat(256), "]".repeat(256));
    let raw = format!(r#"{{"unexpected":{nested}}}"#);
    let error = StoryDraftInsertRequest::from_json(&raw).unwrap_err();
    assert!(error.to_string().contains("recursion limit exceeded"));
}

#[test]
fn project_document_never_implicitly_mutates_or_migrates_revision1() {
    let revision1 = ProjectV2 {
        format: FormatV2,
        schema_revision: SchemaRevisionV1,
        project_id: project_id(1),
        revision: 7,
        meta: ProjectMeta {
            name: "Frozen revision 1".into(),
            version: "0.1".into(),
            author: "test".into(),
        },
        target: generation(1),
        authoring_locales: BTreeSet::new(),
        entities: BTreeMap::new(),
        asset_store: AssetStoreIndex::default(),
    };
    let document = ProjectDocument::Revision1(revision1);
    let original = document.to_canonical_json().unwrap();
    let request = npc_request(&empty_project(), 10, 11, "GoreMods.Npcs.Guard", "GoreGuard");
    let codes = rejection_codes(
        document
            .clone()
            .insert_story_draft(request, ValidationProfile::Experimental)
            .unwrap(),
    );
    assert!(codes.contains(&DiagnosticCode::InvalidStoryMutation));
    assert_eq!(document.to_canonical_json().unwrap(), original);
}
