use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::{
    apply_revision3_npc_draft_transaction_v1, regenerate_revision3_quest_module_v2, AssetMeta,
    AssetStoreIndex, ContentSeal, EntityId, FormatV2, GameGenerationAnchor, ProjectId, ProjectMeta,
    ProjectRevision3, QuestCollisionArtifactRef, QuestCollisionCatalogInput,
    Revision2NpcParentClassInput, Revision3Entity, Revision3EntityKind, Revision3EntityPayload,
    Revision3NpcCatalogAuthorityV1, Revision3NpcCatalogSelectionV1,
    Revision3NpcCollisionAuthorityV1, Revision3NpcCollisionInventoryV1,
    Revision3NpcDraftBuildStatusV1, Revision3NpcDraftInsertConflictV1,
    Revision3NpcDraftInsertEvaluationV1, Revision3NpcDraftInsertOutcomeV1,
    Revision3NpcDraftInsertRequestJsonErrorV1, Revision3NpcDraftInsertRequestV1,
    Revision3NpcDraftIntentV1, Revision3NpcDraftPublicationStatusV1,
    Revision3NpcDraftRuntimeStatusV1, Revision3NpcEntityRoleV1,
    Revision3NpcSourceInspectionStatusV1, Revision3NpcStoryIdentityKindV1, Revision3OriginRef,
    Revision3QuestDraft, Revision3QuestDraftInput, Revision3QuestGiverInput,
    Revision3QuestParentInput, Revision3ScriptModule, Revision3TypedRef, SchemaRevisionV3,
    ScriptModuleStatus, Sha256Digest, WorkingHead, WorkingStoreFormat,
    LOGICAL_NPC_CLONE_GENERATOR_ID, LOGICAL_NPC_CLONE_GENERATOR_VERSION,
    MAX_REVISION3_NPC_DRAFT_REQUEST_JSON_BYTES_V1, QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2,
    QUEST_COLLISION_CATALOG_LAYER_V2, REVISION3_NPC_EXACT_COLLISION_LAYER_V1,
    REVISION3_QUEST_GENERATOR_ID, REVISION3_QUEST_GENERATOR_VERSION,
};
use sha2::{Digest as _, Sha256};

fn id(value: u8) -> EntityId {
    EntityId::from_bytes([value; 16])
}

fn project_id(value: u8) -> ProjectId {
    ProjectId::from_bytes([value; 16])
}

fn seal(value: u8, byte_len: u64) -> ContentSeal {
    ContentSeal {
        byte_len,
        sha256: Sha256Digest::from_bytes([value; 32]),
    }
}

fn target(value: u8) -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: seal(value, 171_698_176),
    }
}

fn head(value: u8) -> WorkingHead {
    WorkingHead {
        store_format: WorkingStoreFormat,
        snapshot: seal(value, 1024),
    }
}

fn project() -> ProjectRevision3 {
    ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id: project_id(0x11),
        revision: 7,
        meta: ProjectMeta {
            name: "R3 NPC transaction".to_owned(),
            version: "1.0.0".to_owned(),
            author: "tests".to_owned(),
        },
        target: target(0x21),
        authoring_locales: BTreeSet::new(),
        entities: BTreeMap::new(),
        asset_store: AssetStoreIndex {
            assets: BTreeMap::new(),
        },
    }
}

fn parent(
    project: &ProjectRevision3,
    seal_value: u8,
    selector_suffix: char,
    runtime_class: &str,
) -> Revision2NpcParentClassInput {
    Revision2NpcParentClassInput {
        generation: project.target.clone(),
        source_seal: seal(seal_value, 20_000 + u64::from(seal_value)),
        catalog_layer: "base-game.g1r.characters".to_owned(),
        canonical_selector: format!("Catalog_{selector_suffix}{}", "0".repeat(62)),
        runtime_class: runtime_class.to_owned(),
    }
}

fn selection(project: &ProjectRevision3) -> Revision3NpcCatalogSelectionV1 {
    Revision3NpcCatalogSelectionV1 {
        generation: project.target.clone(),
        catalog_id: "g1r:npc:om_grd_asghan_263".to_owned(),
        story_catalog_seal: seal(0x31, 5000),
        npc_catalog_seal: seal(0x32, 1_800_000),
        parent_character_definition: parent(
            project,
            0x41,
            '1',
            "UCharacterDefinition_Human_GrdAsghan",
        ),
        parent_ai_agent_config: parent(project, 0x42, '2', "UAIAgentConfig_Human_GrdAsghan"),
        parent_spawn_definition: parent(project, 0x43, '3', "USpawnAIAgentDefinition_GrdAsghan"),
    }
}

fn inventory(project: &ProjectRevision3) -> Revision3NpcCollisionInventoryV1 {
    let basis_head = if project.revision == 8 {
        head(0x72)
    } else {
        head(0x71)
    };
    let project_json = project.to_canonical_json().unwrap();
    let mut inventory = Revision3NpcCollisionInventoryV1 {
        basis_head,
        project_id: project.project_id,
        project_revision: project.revision,
        current_project: ContentSeal {
            byte_len: project_json.len() as u64,
            sha256: Sha256Digest::from_bytes(Sha256::digest(project_json.as_bytes()).into()),
        },
        generation: project.target.clone(),
        story_catalog_seal: seal(0x31, 5000),
        source_seal: seal(0x51, 14_000_000),
        catalog_layer: REVISION3_NPC_EXACT_COLLISION_LAYER_V1.to_owned(),
        catalog_runtime_ids: BTreeSet::new(),
        modules: BTreeSet::new(),
        relative_paths: BTreeSet::new(),
        symbols: BTreeSet::new(),
    };
    for entity in project.entities.values() {
        let (module_id, symbols) = match &entity.payload {
            Revision3EntityPayload::NpcDraft(npc) => (
                npc.script_module.id,
                vec![
                    format!("UCharacterDefinition_Human_{}", npc.input.unique_name),
                    format!("UAIAgentConfig_Human_{}", npc.input.unique_name),
                    format!("USpawnAIAgentDefinition_{}", npc.input.unique_name),
                ],
            ),
            Revision3EntityPayload::QuestDraft(quest) => (
                quest.script_module.id,
                vec![
                    format!("UQuest_{}", quest.input.technical_id),
                    format!("UQuest_{}_OBJ_DONE", quest.input.technical_id),
                    quest.input.text_helper.clone(),
                    "GetGoreExistingQuest".to_owned(),
                    "GetGoreExistingQuestObjective".to_owned(),
                ],
            ),
            _ => continue,
        };
        let Revision3EntityPayload::ScriptModule(module) = &project.entities[&module_id].payload
        else {
            panic!("closed Story draft must reference its module")
        };
        inventory
            .modules
            .insert(module.module_namespace.to_ascii_lowercase());
        inventory
            .relative_paths
            .insert(module.module_relative_path.to_ascii_lowercase());
        for symbol in symbols {
            inventory.symbols.insert(symbol.to_ascii_lowercase());
        }
    }
    inventory
}

fn project_with_valid_quest() -> ProjectRevision3 {
    let mut project = project();
    let quest_id = id(0x40);
    let module_id = id(0x41);
    let artifact = seal(0x81, 4096);
    let source_seal = seal(0x82, artifact.byte_len);
    let artifact_ref = QuestCollisionArtifactRef {
        generation: project.target.clone(),
        catalog_layer: QUEST_COLLISION_CATALOG_LAYER_V2.to_owned(),
        artifact: artifact.clone(),
        source_seal: source_seal.clone(),
        basis_snapshot: seal(0x83, 1024),
    };
    project.asset_store.assets.insert(
        artifact.sha256,
        AssetMeta {
            byte_len: artifact.byte_len,
            media_type: QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2.to_owned(),
        },
    );
    let quest = Revision3QuestDraft {
        generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
        generator_version: REVISION3_QUEST_GENERATOR_VERSION,
        input: Revision3QuestDraftInput {
            target: project.target.clone(),
            quest_id,
            module_namespace: "GoreMods.Quests.ExistingQuest".to_owned(),
            technical_id: "GORE_EXISTING_QUEST".to_owned(),
            text_helper: "ExistingQuestText".to_owned(),
            parent_quest: Revision3QuestParentInput {
                generation: project.target.clone(),
                source_seal: seal(0x84, 5000),
                catalog_layer: "base-game.g1r.quests".to_owned(),
                canonical_selector: "CatalogQuest_SwampCamp_SCCHAPTER2".to_owned(),
                runtime_class: "UQuest_SwampCamp_SCCHAPTER2".to_owned(),
            },
            giver: Revision3QuestGiverInput {
                generation: project.target.clone(),
                source_seal: seal(0x85, 5000),
                catalog_layer: "base-game.g1r.characters".to_owned(),
                canonical_selector: "CatalogCharacter_Asghan".to_owned(),
                runtime_unique_name: "OM_GRD_Asghan_263".to_owned(),
            },
            title: "Existing Quest".to_owned(),
            description: "Already authored before the NPC.".to_owned(),
            objective_title: "Keep the exact Quest closure".to_owned(),
            additional_objective_titles: Vec::new(),
            transition_plan: None,
            collision_catalog: artifact_ref.clone(),
        },
        script_module: Revision3TypedRef::new(
            project.project_id,
            module_id,
            Revision3EntityKind::ScriptModule,
        ),
        transcript: Vec::new(),
    };
    let collision = QuestCollisionCatalogInput {
        generation: project.target.clone(),
        source_seal,
        catalog_layer: artifact_ref.catalog_layer,
        modules: BTreeSet::new(),
        relative_paths: BTreeSet::new(),
        symbols: BTreeSet::new(),
    };
    let module = regenerate_revision3_quest_module_v2(&quest, collision).unwrap();
    let owner = module.owner.clone();
    project.entities.insert(
        quest_id,
        Revision3Entity {
            id: quest_id,
            display_name: "Existing Quest".to_owned(),
            origin: Revision3OriginRef::New {
                authored_runtime_id: quest.input.technical_id.clone(),
            },
            revision: 0,
            payload: Revision3EntityPayload::QuestDraft(quest),
        },
    );
    project.entities.insert(
        module_id,
        Revision3Entity {
            id: module_id,
            display_name: "Existing Quest source".to_owned(),
            origin: Revision3OriginRef::Generated {
                generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
                generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                owner,
            },
            revision: 0,
            payload: Revision3EntityPayload::ScriptModule(module),
        },
    );
    project.validate_closed_model().unwrap();
    project
}

fn request(
    project: &ProjectRevision3,
    basis_head: &WorkingHead,
) -> Revision3NpcDraftInsertRequestV1 {
    Revision3NpcDraftInsertRequestV1 {
        expected_head: basis_head.clone(),
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        expected_target: project.target.clone(),
        npc_id: id(0x61),
        script_module_id: id(0x62),
        display_name: "Gate guard".to_owned(),
        intent: Revision3NpcDraftIntentV1 {
            module_namespace: "GoreMods.Npcs.GateGuard".to_owned(),
            unique_name: "GORE_GATE_GUARD".to_owned(),
            parent_catalog_id: "g1r:npc:om_grd_asghan_263".to_owned(),
        },
    }
}

fn evaluate(
    project: &ProjectRevision3,
    basis_head: &WorkingHead,
    request: &Revision3NpcDraftInsertRequestV1,
    selection: Revision3NpcCatalogSelectionV1,
    inventory: Revision3NpcCollisionInventoryV1,
) -> Revision3NpcDraftInsertEvaluationV1 {
    apply_revision3_npc_draft_transaction_v1(
        basis_head,
        &project.to_canonical_json().unwrap(),
        &request.to_canonical_json().unwrap(),
        selection,
        inventory,
    )
    .unwrap()
}

fn applied(value: Revision3NpcDraftInsertEvaluationV1) -> Revision3NpcDraftInsertOutcomeV1 {
    match value {
        Revision3NpcDraftInsertEvaluationV1::Applied(value) => *value,
        Revision3NpcDraftInsertEvaluationV1::Rejected(value) => {
            panic!("unexpected rejection: {}", value.conflict)
        }
    }
}

fn rejected(value: Revision3NpcDraftInsertEvaluationV1) -> Revision3NpcDraftInsertConflictV1 {
    match value {
        Revision3NpcDraftInsertEvaluationV1::Rejected(value) => value.conflict,
        Revision3NpcDraftInsertEvaluationV1::Applied(_) => panic!("unexpected candidate"),
    }
}

#[test]
fn happy_insert_is_atomic_deterministic_reopened_and_permanently_unqualified() {
    let project = project();
    let basis_head = head(0x71);
    let request = request(&project, &basis_head);
    let first = applied(evaluate(
        &project,
        &basis_head,
        &request,
        selection(&project),
        inventory(&project),
    ));
    let second = applied(evaluate(
        &project,
        &basis_head,
        &request,
        selection(&project),
        inventory(&project),
    ));
    assert_eq!(first, second);
    assert_eq!(first.project.revision, 8);
    assert_eq!(first.project.entities.len(), 2);
    assert_eq!(first.basis_head, basis_head);
    assert_eq!(first.build_status, Revision3NpcDraftBuildStatusV1::Blocked);
    assert_eq!(
        first.runtime_status,
        Revision3NpcDraftRuntimeStatusV1::RuntimeUnqualified
    );
    assert_eq!(
        first.catalog_authority,
        Revision3NpcCatalogAuthorityV1::NotGranted
    );
    assert_eq!(
        first.collision_authority,
        Revision3NpcCollisionAuthorityV1::NotGranted
    );
    assert_eq!(
        first.source_inspection,
        Revision3NpcSourceInspectionStatusV1::FreshNativeContextRequired
    );
    assert_eq!(
        first.publication_status,
        Revision3NpcDraftPublicationStatusV1::NotSupported
    );
    assert_eq!(
        ProjectRevision3::from_json(&first.canonical_project_json).unwrap(),
        first.project
    );

    let npc_entity = &first.project.entities[&request.npc_id];
    let Revision3EntityPayload::NpcDraft(npc) = &npc_entity.payload else {
        panic!("expected NPC Draft")
    };
    let module_entity = &first.project.entities[&request.script_module_id];
    let Revision3EntityPayload::ScriptModule(module) = &module_entity.payload else {
        panic!("expected ScriptModule")
    };
    assert_eq!(npc.generator_id, LOGICAL_NPC_CLONE_GENERATOR_ID);
    assert_eq!(npc.generator_version, LOGICAL_NPC_CLONE_GENERATOR_VERSION);
    assert_eq!(npc.input.target, project.target);
    assert_eq!(npc.input.unique_name, "GORE_GATE_GUARD");
    assert_eq!(
        npc.script_module,
        Revision3TypedRef::new(
            project.project_id,
            request.script_module_id,
            Revision3EntityKind::ScriptModule,
        )
    );
    assert!(matches!(
        &npc_entity.origin,
        Revision3OriginRef::New { authored_runtime_id }
            if authored_runtime_id == "GORE_GATE_GUARD"
    ));
    assert_eq!(
        module.status,
        ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED
    );
    assert_eq!(module.owner.id, request.npc_id);
    assert_eq!(module.owner.expected_kind, Revision3EntityKind::NpcDraft);
    assert_eq!(
        module.source_sha256,
        Sha256Digest::from_bytes(Sha256::digest(module.source.as_bytes()).into())
    );
    assert!(module
        .source
        .contains("UCharacterDefinition_Human_GORE_GATE_GUARD"));
}

#[test]
fn request_wire_is_bounded_canonical_and_duplicate_safe() {
    let project = project();
    let request = request(&project, &head(0x71));
    let canonical = request.to_canonical_json().unwrap();
    assert_eq!(
        Revision3NpcDraftInsertRequestV1::from_json(&canonical).unwrap(),
        request
    );
    assert!(matches!(
        Revision3NpcDraftInsertRequestV1::from_json(&format!(" {canonical}")),
        Err(Revision3NpcDraftInsertRequestJsonErrorV1::NonCanonicalJson)
    ));
    let duplicate = canonical.replacen(
        "\"expected_revision\":7",
        "\"expected_revision\":7,\"expected_revision\":7",
        1,
    );
    assert!(matches!(
        Revision3NpcDraftInsertRequestV1::from_json(&duplicate),
        Err(Revision3NpcDraftInsertRequestJsonErrorV1::InvalidJson(_))
    ));
    let nested_duplicate = canonical.replacen(
        "\"unique_name\":\"GORE_GATE_GUARD\"",
        "\"unique_name\":\"GORE_GATE_GUARD\",\"unique_name\":\"GORE_GATE_GUARD\"",
        1,
    );
    assert!(matches!(
        Revision3NpcDraftInsertRequestV1::from_json(&nested_duplicate),
        Err(Revision3NpcDraftInsertRequestJsonErrorV1::InvalidJson(_))
    ));
    let mut oversized = request;
    oversized.display_name = "x".repeat(MAX_REVISION3_NPC_DRAFT_REQUEST_JSON_BYTES_V1);
    assert!(matches!(
        oversized.to_canonical_json(),
        Err(Revision3NpcDraftInsertRequestJsonErrorV1::InputTooLarge { .. })
    ));
}

#[test]
fn stale_foreign_target_and_head_bindings_return_no_candidate() {
    let project = project();
    let basis_head = head(0x71);
    let base_json = project.to_canonical_json().unwrap();

    let mut stale = request(&project, &basis_head);
    stale.expected_revision -= 1;
    assert!(matches!(
        rejected(evaluate(
            &project,
            &basis_head,
            &stale,
            selection(&project),
            inventory(&project),
        )),
        Revision3NpcDraftInsertConflictV1::ProjectRevisionConflict { .. }
    ));
    let mut foreign = request(&project, &basis_head);
    foreign.expected_project_id = project_id(0x99);
    assert!(matches!(
        rejected(evaluate(
            &project,
            &basis_head,
            &foreign,
            selection(&project),
            inventory(&project),
        )),
        Revision3NpcDraftInsertConflictV1::ProjectIdentityMismatch { .. }
    ));
    let mut wrong_target = request(&project, &basis_head);
    wrong_target.expected_target = target(0x99);
    assert!(matches!(
        rejected(evaluate(
            &project,
            &basis_head,
            &wrong_target,
            selection(&project),
            inventory(&project),
        )),
        Revision3NpcDraftInsertConflictV1::ProjectTargetMismatch
    ));
    let wrong_head = head(0x72);
    assert!(matches!(
        rejected(evaluate(
            &project,
            &wrong_head,
            &request(&project, &basis_head),
            selection(&project),
            inventory(&project),
        )),
        Revision3NpcDraftInsertConflictV1::CurrentHeadMismatch
    ));
    assert_eq!(project.to_canonical_json().unwrap(), base_json);
}

#[test]
fn entity_shape_catalog_and_inventory_drift_fail_closed() {
    let project = project();
    let basis_head = head(0x71);

    let mut shared = request(&project, &basis_head);
    shared.script_module_id = shared.npc_id;
    assert!(matches!(
        rejected(evaluate(
            &project,
            &basis_head,
            &shared,
            selection(&project),
            inventory(&project),
        )),
        Revision3NpcDraftInsertConflictV1::SharedEntityId
    ));

    let mut wrong_selection = selection(&project);
    wrong_selection.catalog_id = "g1r:npc:other".to_owned();
    assert!(matches!(
        rejected(evaluate(
            &project,
            &basis_head,
            &request(&project, &basis_head),
            wrong_selection,
            inventory(&project),
        )),
        Revision3NpcDraftInsertConflictV1::CatalogSelectionMismatch
    ));

    let mut wrong_selection = selection(&project);
    wrong_selection.parent_spawn_definition.generation = target(0x90);
    assert!(matches!(
        rejected(evaluate(
            &project,
            &basis_head,
            &request(&project, &basis_head),
            wrong_selection,
            inventory(&project),
        )),
        Revision3NpcDraftInsertConflictV1::InvalidCatalogSelection
    ));

    let mut wrong_inventory = inventory(&project);
    wrong_inventory.story_catalog_seal.sha256 = Sha256Digest::from_bytes([0x77; 32]);
    assert!(matches!(
        rejected(evaluate(
            &project,
            &basis_head,
            &request(&project, &basis_head),
            selection(&project),
            wrong_inventory,
        )),
        Revision3NpcDraftInsertConflictV1::CollisionStoryCatalogMismatch
    ));

    let mut wrong_inventory = inventory(&project);
    wrong_inventory.catalog_layer = "resolved-loadout.scripts.v1".to_owned();
    assert!(matches!(
        rejected(evaluate(
            &project,
            &basis_head,
            &request(&project, &basis_head),
            selection(&project),
            wrong_inventory,
        )),
        Revision3NpcDraftInsertConflictV1::CollisionLayerMismatch
    ));

    let mut malformed_inventory = inventory(&project);
    malformed_inventory.symbols.insert("non-ascii-λ".to_owned());
    assert!(matches!(
        rejected(evaluate(
            &project,
            &basis_head,
            &request(&project, &basis_head),
            selection(&project),
            malformed_inventory,
        )),
        Revision3NpcDraftInsertConflictV1::InvalidCollisionInventory
    ));
}

#[test]
fn collision_inventory_must_match_the_exact_head_and_project_bytes() {
    let project = project();
    let basis_head = head(0x71);
    let request = request(&project, &basis_head);

    let mut wrong_head = inventory(&project);
    wrong_head.basis_head = head(0x72);
    assert!(matches!(
        rejected(evaluate(
            &project,
            &basis_head,
            &request,
            selection(&project),
            wrong_head,
        )),
        Revision3NpcDraftInsertConflictV1::CollisionBasisMismatch
    ));

    let mut wrong_project = inventory(&project);
    wrong_project.project_id = project_id(0x99);
    assert!(matches!(
        rejected(evaluate(
            &project,
            &basis_head,
            &request,
            selection(&project),
            wrong_project,
        )),
        Revision3NpcDraftInsertConflictV1::CollisionBasisMismatch
    ));

    let mut wrong_revision = inventory(&project);
    wrong_revision.project_revision -= 1;
    assert!(matches!(
        rejected(evaluate(
            &project,
            &basis_head,
            &request,
            selection(&project),
            wrong_revision,
        )),
        Revision3NpcDraftInsertConflictV1::CollisionBasisMismatch
    ));

    let mut wrong_bytes = inventory(&project);
    wrong_bytes.current_project.sha256 = Sha256Digest::from_bytes([0x98; 32]);
    assert!(matches!(
        rejected(evaluate(
            &project,
            &basis_head,
            &request,
            selection(&project),
            wrong_bytes,
        )),
        Revision3NpcDraftInsertConflictV1::CollisionBasisMismatch
    ));
}

#[test]
fn native_module_path_and_symbol_collisions_are_distinct() {
    let project = project();
    let basis_head = head(0x71);
    let cases = [
        (
            Revision3NpcStoryIdentityKindV1::ModuleNamespace,
            "goremods.npcs.gateguard",
            0,
        ),
        (
            Revision3NpcStoryIdentityKindV1::ModuleRelativePath,
            "goremods/npcs/gateguard.as",
            1,
        ),
        (
            Revision3NpcStoryIdentityKindV1::GeneratedSymbol,
            "ucharacterdefinition_human_gore_gate_guard",
            2,
        ),
    ];
    for (expected_kind, value, domain) in cases {
        let mut inventory = inventory(&project);
        match domain {
            0 => assert!(inventory.modules.insert(value.to_owned())),
            1 => assert!(inventory.relative_paths.insert(value.to_owned())),
            _ => assert!(inventory.symbols.insert(value.to_owned())),
        }
        assert!(matches!(
            rejected(evaluate(
                &project,
                &basis_head,
                &request(&project, &basis_head),
                selection(&project),
                inventory,
            )),
            Revision3NpcDraftInsertConflictV1::StoryIdentityCollision {
                kind,
                existing_entity: None,
                ..
            } if kind == expected_kind
        ));
    }
}

#[test]
fn fresh_catalog_runtime_identity_collision_is_explicit_and_case_insensitive() {
    let project = project();
    let basis_head = head(0x71);
    let request = request(&project, &basis_head);
    let mut inventory = inventory(&project);
    assert!(inventory
        .catalog_runtime_ids
        .insert("gore_gate_guard".to_owned()));

    assert!(matches!(
        rejected(evaluate(
            &project,
            &basis_head,
            &request,
            selection(&project),
            inventory,
        )),
        Revision3NpcDraftInsertConflictV1::StoryIdentityCollision {
            kind: Revision3NpcStoryIdentityKindV1::AuthoredRuntimeId,
            existing_entity: None,
            ..
        }
    ));
}

#[test]
fn exact_project_runtime_and_generated_identity_collisions_are_rejected() {
    let project = project();
    let first_head = head(0x71);
    let first_request = request(&project, &first_head);
    let first = applied(evaluate(
        &project,
        &first_head,
        &first_request,
        selection(&project),
        inventory(&project),
    ));
    let second_head = head(0x72);

    let mut runtime = request(&first.project, &second_head);
    runtime.npc_id = id(0x63);
    runtime.script_module_id = id(0x64);
    runtime.intent.module_namespace = "GoreMods.Npcs.Other".to_owned();
    assert!(matches!(
        rejected(evaluate(
            &first.project,
            &second_head,
            &runtime,
            selection(&first.project),
            inventory(&first.project),
        )),
        Revision3NpcDraftInsertConflictV1::StoryIdentityCollision {
            kind: Revision3NpcStoryIdentityKindV1::AuthoredRuntimeId,
            existing_entity: Some(existing),
            ..
        } if existing == first_request.npc_id
    ));

    let mut module = request(&first.project, &second_head);
    module.npc_id = id(0x65);
    module.script_module_id = id(0x66);
    module.intent.unique_name = "GORE_OTHER_GUARD".to_owned();
    assert!(matches!(
        rejected(evaluate(
            &first.project,
            &second_head,
            &module,
            selection(&first.project),
            inventory(&first.project),
        )),
        Revision3NpcDraftInsertConflictV1::StoryIdentityCollision {
            kind: Revision3NpcStoryIdentityKindV1::ModuleNamespace,
            existing_entity: None,
            ..
        }
    ));
}

#[test]
fn valid_existing_revision3_quest_is_preserved_while_npc_pair_is_inserted() {
    let project = project_with_valid_quest();
    let basis_head = head(0x71);
    let request = request(&project, &basis_head);
    let quest_before = project.entities[&id(0x40)].clone();
    let module_before = project.entities[&id(0x41)].clone();
    let outcome = applied(evaluate(
        &project,
        &basis_head,
        &request,
        selection(&project),
        inventory(&project),
    ));

    assert_eq!(outcome.project.revision, project.revision + 1);
    assert_eq!(outcome.project.entities.len(), project.entities.len() + 2);
    assert_eq!(outcome.project.entities[&id(0x40)], quest_before);
    assert_eq!(outcome.project.entities[&id(0x41)], module_before);
    outcome.project.validate_closed_model().unwrap();
}

#[test]
fn revision_overflow_invalid_intent_and_residual_quest_state_return_no_candidate() {
    let mut overflow = project();
    overflow.revision = u64::MAX;
    let basis_head = head(0x71);
    assert!(matches!(
        rejected(evaluate(
            &overflow,
            &basis_head,
            &request(&overflow, &basis_head),
            selection(&overflow),
            inventory(&overflow),
        )),
        Revision3NpcDraftInsertConflictV1::ProjectRevisionOverflow
    ));

    let base_project = project();
    let mut invalid = request(&base_project, &basis_head);
    invalid.intent.unique_name = "bad-name".to_owned();
    assert!(matches!(
        rejected(evaluate(
            &base_project,
            &basis_head,
            &invalid,
            selection(&base_project),
            inventory(&base_project),
        )),
        Revision3NpcDraftInsertConflictV1::InvalidNpcIntent { .. }
    ));

    let mut residual = project();
    let source = "class Residual {}".to_owned();
    residual.entities.insert(
        id(0x41),
        gore_authoring::Revision3Entity {
            id: id(0x41),
            display_name: "Residual Quest source".to_owned(),
            origin: Revision3OriginRef::Generated {
                generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
                generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                owner: Revision3TypedRef::new(
                    residual.project_id,
                    id(0x40),
                    Revision3EntityKind::QuestDraft,
                ),
            },
            revision: 0,
            payload: Revision3EntityPayload::ScriptModule(Revision3ScriptModule {
                generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
                generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                owner: Revision3TypedRef::new(
                    residual.project_id,
                    id(0x40),
                    Revision3EntityKind::QuestDraft,
                ),
                module_namespace: "Residual.Quest".to_owned(),
                module_relative_path: "Residual/Quest.as".to_owned(),
                source_sha256: Sha256Digest::from_bytes(Sha256::digest(source.as_bytes()).into()),
                source,
                input_fingerprint: Sha256Digest::from_bytes([0x81; 32]),
                status: ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED,
            }),
        },
    );
    assert!(matches!(
        rejected(evaluate(
            &residual,
            &basis_head,
            &request(&residual, &basis_head),
            selection(&residual),
            inventory(&residual),
        )),
        Revision3NpcDraftInsertConflictV1::InvalidBasisStoryState { .. }
    ));
}

#[test]
fn closed_model_rejects_generated_npc_source_drift_and_orphans() {
    let project = project();
    let basis_head = head(0x71);
    let request = request(&project, &basis_head);
    let applied = applied(evaluate(
        &project,
        &basis_head,
        &request,
        selection(&project),
        inventory(&project),
    ));

    let mut drift = applied.project.clone();
    let Revision3EntityPayload::ScriptModule(module) = &mut drift
        .entities
        .get_mut(&request.script_module_id)
        .unwrap()
        .payload
    else {
        panic!("expected module")
    };
    module.source.push_str("\n// drift");
    assert!(matches!(
        drift.validate_closed_model(),
        Err(gore_authoring::ProjectRevision3ValidationError::InvalidNpcScriptReference { .. })
    ));

    let mut orphan = applied.project;
    orphan.entities.remove(&request.npc_id);
    assert!(matches!(
        orphan.validate_closed_model(),
        Err(gore_authoring::ProjectRevision3ValidationError::OrphanNpcScriptModule { .. })
    ));
}

#[test]
fn zero_and_colliding_entity_ids_are_role_specific() {
    let project = project();
    let basis_head = head(0x71);
    let mut zero = request(&project, &basis_head);
    zero.npc_id = EntityId::from_bytes([0; 16]);
    assert!(matches!(
        rejected(evaluate(
            &project,
            &basis_head,
            &zero,
            selection(&project),
            inventory(&project),
        )),
        Revision3NpcDraftInsertConflictV1::ZeroEntityId {
            role: Revision3NpcEntityRoleV1::NpcDraft
        }
    ));
}
