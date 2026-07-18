use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::{
    apply_revision3_quest_draft_transaction_v2, regenerate_revision3_quest_module,
    revision3_quest_input_fingerprint, AssetMeta, AssetStoreIndex, ContentSeal,
    DraftQuestCollisionKind, DraftQuestSkeletonError, EntityId, FormatV2, GameGenerationAnchor,
    NpcParentClassInput, ProjectId, ProjectMeta, ProjectRevision3, QuestCollisionArtifactRef,
    QuestCollisionCatalogInput, Revision3Entity, Revision3EntityKind, Revision3EntityPayload,
    Revision3NpcDraft, Revision3NpcDraftInput, Revision3OriginRef,
    Revision3QuestArtifactAuthorityV2, Revision3QuestDraftBuildStatusV2,
    Revision3QuestDraftInsertConflictV2, Revision3QuestDraftInsertErrorV2,
    Revision3QuestDraftInsertEvaluationV2, Revision3QuestDraftInsertOutcomeV2,
    Revision3QuestDraftInsertRequestJsonErrorV2, Revision3QuestDraftInsertRequestV2,
    Revision3QuestDraftIntentV2, Revision3QuestDraftRuntimeStatusV2, Revision3QuestEntityRoleV2,
    Revision3QuestGenerationError, Revision3QuestGiverInput, Revision3QuestParentInput,
    Revision3QuestSourceInspectionStatusV2, Revision3StoryIdentityKindV2, Revision3TypedRef,
    SchemaRevisionV3, ScriptModuleStatus, Sha256Digest, LOGICAL_NPC_CLONE_GENERATOR_ID,
    LOGICAL_NPC_CLONE_GENERATOR_VERSION, MAX_REVISION3_QUEST_DRAFT_REQUEST_JSON_BYTES,
    MAX_REVISION3_SNAPSHOT_BYTES, QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2,
    QUEST_COLLISION_CATALOG_LAYER_V2, REVISION3_QUEST_GENERATOR_ID,
    REVISION3_QUEST_GENERATOR_VERSION,
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

fn target(value: u8) -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: seal(value, 171_698_176),
    }
}

fn artifact_reference(project: &ProjectRevision3) -> QuestCollisionArtifactRef {
    QuestCollisionArtifactRef {
        generation: project.target.clone(),
        catalog_layer: QUEST_COLLISION_CATALOG_LAYER_V2.to_owned(),
        artifact: seal(0x41, 3_517_569),
        source_seal: seal(0x42, 3_517_569),
        basis_snapshot: seal(0x43, 640),
    }
}

fn empty_basis() -> ProjectRevision3 {
    let mut project = ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id: project_id(1),
        revision: 7,
        meta: ProjectMeta {
            name: "S4 Quest transaction".to_owned(),
            version: "0.1.0".to_owned(),
            author: "tests".to_owned(),
        },
        target: target(1),
        authoring_locales: BTreeSet::new(),
        entities: BTreeMap::new(),
        asset_store: AssetStoreIndex::default(),
    };
    let reference = artifact_reference(&project);
    project.asset_store.assets.insert(
        reference.artifact.sha256,
        AssetMeta {
            byte_len: reference.artifact.byte_len,
            media_type: QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2.to_owned(),
        },
    );
    project
}

fn parent(project: &ProjectRevision3) -> Revision3QuestParentInput {
    Revision3QuestParentInput {
        generation: project.target.clone(),
        source_seal: seal(0x21, 20_000),
        catalog_layer: "base-game.g1r.quests".to_owned(),
        canonical_selector: "CatalogQuest_SwampCamp_SCCHAPTER2".to_owned(),
        runtime_class: "UQuest_SwampCamp_SCCHAPTER2".to_owned(),
    }
}

fn giver(project: &ProjectRevision3) -> Revision3QuestGiverInput {
    Revision3QuestGiverInput {
        generation: project.target.clone(),
        source_seal: seal(0x22, 30_000),
        catalog_layer: "base-game.g1r.characters".to_owned(),
        canonical_selector: "CatalogCharacter_Asghan".to_owned(),
        runtime_unique_name: "OM_GRD_Asghan_263".to_owned(),
    }
}

fn request(project: &ProjectRevision3) -> Revision3QuestDraftInsertRequestV2 {
    Revision3QuestDraftInsertRequestV2 {
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        expected_target: project.target.clone(),
        quest_id: entity_id(0x51),
        script_module_id: entity_id(0x52),
        display_name: "Asghan's S4 Trial".to_owned(),
        intent: Revision3QuestDraftIntentV2 {
            module_namespace: "GoreMods.Quests.S4AsghanTrial".to_owned(),
            technical_id: "GORE_S4_ASGHAN_TRIAL".to_owned(),
            text_helper: "GoreS4QuestText".to_owned(),
            parent_quest: parent(project),
            giver: giver(project),
            title: "Asghan's Trial".to_owned(),
            description: "Prove that the gate remains secure.".to_owned(),
            objective_title: "Report to Asghan".to_owned(),
            collision_catalog: artifact_reference(project),
        },
    }
}

fn collision_input(project: &ProjectRevision3) -> QuestCollisionCatalogInput {
    let reference = artifact_reference(project);
    QuestCollisionCatalogInput {
        generation: reference.generation,
        source_seal: reference.source_seal,
        catalog_layer: reference.catalog_layer,
        modules: BTreeSet::new(),
        relative_paths: BTreeSet::new(),
        symbols: BTreeSet::new(),
    }
}

fn evaluate(
    project: &ProjectRevision3,
    request: &Revision3QuestDraftInsertRequestV2,
    collision: QuestCollisionCatalogInput,
) -> Revision3QuestDraftInsertEvaluationV2 {
    apply_revision3_quest_draft_transaction_v2(
        &project.to_canonical_json().unwrap(),
        &request.to_canonical_json().unwrap(),
        collision,
    )
    .unwrap()
}

fn applied(
    evaluation: Revision3QuestDraftInsertEvaluationV2,
) -> Revision3QuestDraftInsertOutcomeV2 {
    match evaluation {
        Revision3QuestDraftInsertEvaluationV2::Applied(outcome) => *outcome,
        Revision3QuestDraftInsertEvaluationV2::Rejected(rejection) => {
            panic!("unexpected S4 rejection: {}", rejection.conflict)
        }
    }
}

fn rejected(
    evaluation: Revision3QuestDraftInsertEvaluationV2,
) -> Revision3QuestDraftInsertConflictV2 {
    match evaluation {
        Revision3QuestDraftInsertEvaluationV2::Rejected(rejection) => rejection.conflict,
        Revision3QuestDraftInsertEvaluationV2::Applied(_) => panic!("unexpected S4 candidate"),
    }
}

#[test]
fn happy_insert_is_atomic_deterministic_canonical_and_permanently_unqualified() {
    let project = empty_basis();
    let request = request(&project);
    let base_json = project.to_canonical_json().unwrap();
    let asset_store = project.asset_store.clone();

    let first = applied(evaluate(&project, &request, collision_input(&project)));
    let second = applied(evaluate(&project, &request, collision_input(&project)));
    assert_eq!(first, second);
    assert_eq!(first.project.revision, project.revision + 1);
    assert_eq!(first.project.entities.len(), project.entities.len() + 2);
    assert_eq!(first.project.asset_store, asset_store);
    assert_eq!(project.to_canonical_json().unwrap(), base_json);
    assert_eq!(
        first.build_status,
        Revision3QuestDraftBuildStatusV2::Blocked
    );
    assert_eq!(
        first.runtime_status,
        Revision3QuestDraftRuntimeStatusV2::RuntimeUnqualified
    );
    assert_eq!(
        first.artifact_authority,
        Revision3QuestArtifactAuthorityV2::NotGranted
    );
    assert_eq!(
        first.source_inspection,
        Revision3QuestSourceInspectionStatusV2::FreshCapabilityRequired
    );
    assert_eq!(
        ProjectRevision3::from_json(&first.canonical_project_json).unwrap(),
        first.project
    );

    let quest_entity = &first.project.entities[&request.quest_id];
    let Revision3EntityPayload::QuestDraft(quest) = &quest_entity.payload else {
        panic!("expected S4 Quest Draft")
    };
    let module_entity = &first.project.entities[&request.script_module_id];
    let Revision3EntityPayload::ScriptModule(module) = &module_entity.payload else {
        panic!("expected S4 ScriptModule")
    };
    assert_eq!(quest.generator_id, REVISION3_QUEST_GENERATOR_ID);
    assert_eq!(quest.generator_version, REVISION3_QUEST_GENERATOR_VERSION);
    assert_eq!(quest.input.quest_id, request.quest_id);
    assert_eq!(quest.input.target, project.target);
    assert_eq!(
        quest.script_module,
        Revision3TypedRef::new(
            project.project_id,
            request.script_module_id,
            Revision3EntityKind::ScriptModule,
        )
    );
    assert!(matches!(
        &quest_entity.origin,
        Revision3OriginRef::New { authored_runtime_id }
            if authored_runtime_id == &quest.input.technical_id
    ));
    assert_eq!(module.generator_id, REVISION3_QUEST_GENERATOR_ID);
    assert_eq!(module.generator_version, REVISION3_QUEST_GENERATOR_VERSION);
    assert_eq!(module.owner.id, request.quest_id);
    assert_eq!(module.owner.expected_kind, Revision3EntityKind::QuestDraft);
    assert_eq!(
        module.status,
        ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED
    );
    assert!(matches!(
        &module_entity.origin,
        Revision3OriginRef::Generated {
            generator_id,
            generator_version,
            owner,
        } if generator_id == REVISION3_QUEST_GENERATOR_ID
            && *generator_version == REVISION3_QUEST_GENERATOR_VERSION
            && owner == &module.owner
    ));
    assert_eq!(
        module.input_fingerprint,
        revision3_quest_input_fingerprint(&quest.input).unwrap()
    );
    // Keep one fixed v4 digest so a future refactor cannot silently drift the current contract.
    assert_eq!(
        module.input_fingerprint.to_string(),
        "963aa38baf276d98ad34ba1dbd79ba380c60209bb2ae519c94726226ded3e1b4"
    );
    assert_eq!(
        module.source_sha256,
        Sha256Digest::from_bytes(Sha256::digest(module.source.as_bytes()).into())
    );
    assert!(module.source.contains("UQuest_GORE_S4_ASGHAN_TRIAL"));
}

#[test]
fn request_wire_is_bounded_exact_canonical_and_duplicate_safe() {
    let project = empty_basis();
    let request = request(&project);
    let canonical = request.to_canonical_json().unwrap();
    assert_eq!(
        Revision3QuestDraftInsertRequestV2::from_json(&canonical).unwrap(),
        request
    );
    assert!(matches!(
        Revision3QuestDraftInsertRequestV2::from_json(&format!(" {canonical}")),
        Err(Revision3QuestDraftInsertRequestJsonErrorV2::NonCanonicalJson)
    ));
    let duplicate = canonical.replacen(
        "\"expected_revision\":7",
        "\"expected_revision\":7,\"expected_revision\":7",
        1,
    );
    assert!(matches!(
        Revision3QuestDraftInsertRequestV2::from_json(&duplicate),
        Err(Revision3QuestDraftInsertRequestJsonErrorV2::InvalidJson(_))
    ));
    let nested_duplicate = canonical.replacen(
        "\"technical_id\":\"GORE_S4_ASGHAN_TRIAL\"",
        "\"technical_id\":\"GORE_S4_ASGHAN_TRIAL\",\"technical_id\":\"GORE_S4_ASGHAN_TRIAL\"",
        1,
    );
    assert!(matches!(
        Revision3QuestDraftInsertRequestV2::from_json(&nested_duplicate),
        Err(Revision3QuestDraftInsertRequestJsonErrorV2::InvalidJson(_))
    ));
    let mut oversized = request;
    oversized.intent.description = "x".repeat(MAX_REVISION3_QUEST_DRAFT_REQUEST_JSON_BYTES);
    assert!(matches!(
        oversized.to_canonical_json(),
        Err(Revision3QuestDraftInsertRequestJsonErrorV2::InputTooLarge { .. })
    ));
}

#[test]
fn stale_foreign_and_target_conflicts_return_no_candidate_and_leave_base_exact() {
    let project = empty_basis();
    let base_json = project.to_canonical_json().unwrap();

    let mut stale = request(&project);
    stale.expected_revision -= 1;
    assert!(matches!(
        rejected(evaluate(&project, &stale, collision_input(&project))),
        Revision3QuestDraftInsertConflictV2::ProjectRevisionConflict { .. }
    ));

    let mut foreign = request(&project);
    foreign.expected_project_id = project_id(9);
    assert!(matches!(
        rejected(evaluate(&project, &foreign, collision_input(&project))),
        Revision3QuestDraftInsertConflictV2::ProjectIdentityMismatch { .. }
    ));

    let mut target_drift = request(&project);
    target_drift.expected_target = target(9);
    assert!(matches!(
        rejected(evaluate(&project, &target_drift, collision_input(&project))),
        Revision3QuestDraftInsertConflictV2::ProjectTargetMismatch
    ));

    assert_eq!(project.to_canonical_json().unwrap(), base_json);
    assert!(ProjectRevision3::from_json(&base_json)
        .unwrap()
        .entities
        .is_empty());
}

#[test]
fn entity_and_sealed_catalog_collisions_fail_closed() {
    let project = empty_basis();

    let mut same_id = request(&project);
    same_id.script_module_id = same_id.quest_id;
    assert!(matches!(
        rejected(evaluate(&project, &same_id, collision_input(&project))),
        Revision3QuestDraftInsertConflictV2::SharedEntityId
    ));

    let cases = [
        (
            Revision3StoryIdentityKindV2::ModuleNamespace,
            DraftQuestCollisionKind::Module,
            "GoreMods.Quests.S4AsghanTrial",
        ),
        (
            Revision3StoryIdentityKindV2::ModuleRelativePath,
            DraftQuestCollisionKind::RelativePath,
            "GoreMods/Quests/S4AsghanTrial.as",
        ),
        (
            Revision3StoryIdentityKindV2::GeneratedSymbol,
            DraftQuestCollisionKind::Symbol,
            "UQuest_GORE_S4_ASGHAN_TRIAL",
        ),
    ];
    for (expected_kind, catalog_kind, value) in cases {
        let mut collision = collision_input(&project);
        match catalog_kind {
            DraftQuestCollisionKind::Module => {
                collision.modules.insert(value.to_owned());
            }
            DraftQuestCollisionKind::RelativePath => {
                collision.relative_paths.insert(value.to_owned());
            }
            DraftQuestCollisionKind::Symbol => {
                collision.symbols.insert(value.to_owned());
            }
        }
        assert!(matches!(
            rejected(evaluate(&project, &request(&project), collision)),
            Revision3QuestDraftInsertConflictV2::StoryIdentityCollision {
                kind,
                existing_entity: None,
                ..
            } if kind == expected_kind
        ));
    }
}

#[test]
fn artifact_reference_and_caller_collision_provenance_drift_are_distinct() {
    let project = empty_basis();

    let mut generation_ref = request(&project);
    generation_ref.intent.collision_catalog.generation = target(8);
    assert!(matches!(
        rejected(evaluate(
            &project,
            &generation_ref,
            collision_input(&project)
        )),
        Revision3QuestDraftInsertConflictV2::ArtifactGenerationMismatch
    ));

    let mut bad_basis = request(&project);
    bad_basis.intent.collision_catalog.basis_snapshot.byte_len = MAX_REVISION3_SNAPSHOT_BYTES + 1;
    assert!(matches!(
        rejected(evaluate(&project, &bad_basis, collision_input(&project))),
        Revision3QuestDraftInsertConflictV2::InvalidBasisSnapshot
    ));

    let mut bad_layer = request(&project);
    bad_layer.intent.collision_catalog.catalog_layer = "foreign.story-collisions.v1".to_owned();
    assert!(matches!(
        rejected(evaluate(&project, &bad_layer, collision_input(&project))),
        Revision3QuestDraftInsertConflictV2::ArtifactCatalogLayerMismatch
    ));

    let mut input = collision_input(&project);
    input.generation = target(8);
    assert!(matches!(
        rejected(evaluate(&project, &request(&project), input)),
        Revision3QuestDraftInsertConflictV2::CollisionInputGenerationMismatch
    ));
    let mut input = collision_input(&project);
    input.source_seal.sha256 = Sha256Digest::from_bytes([0x81; 32]);
    assert!(matches!(
        rejected(evaluate(&project, &request(&project), input)),
        Revision3QuestDraftInsertConflictV2::CollisionInputSourceSealMismatch
    ));
    let mut input = collision_input(&project);
    input.catalog_layer = "foreign.story-collisions.v1".to_owned();
    assert!(matches!(
        rejected(evaluate(&project, &request(&project), input)),
        Revision3QuestDraftInsertConflictV2::CollisionInputCatalogLayerMismatch
    ));
}

#[test]
fn shared_generator_binds_every_collision_provenance_field_to_the_artifact_ref() {
    let project = empty_basis();
    let request = request(&project);
    let applied = applied(evaluate(&project, &request, collision_input(&project)));
    let Revision3EntityPayload::QuestDraft(quest) =
        &applied.project.entities[&request.quest_id].payload
    else {
        panic!("expected S4 Quest Draft")
    };

    let mut wrong_generation = collision_input(&project);
    wrong_generation.generation = target(8);
    assert!(matches!(
        regenerate_revision3_quest_module(quest, wrong_generation),
        Err(Revision3QuestGenerationError::CollisionGenerationMismatch)
    ));

    let mut wrong_source = collision_input(&project);
    wrong_source.source_seal.sha256 = Sha256Digest::from_bytes([0x81; 32]);
    assert!(matches!(
        regenerate_revision3_quest_module(quest, wrong_source),
        Err(Revision3QuestGenerationError::CollisionSourceSealMismatch)
    ));

    let mut wrong_layer = collision_input(&project);
    wrong_layer.catalog_layer = "unsupported.story-collisions".to_owned();
    assert!(matches!(
        regenerate_revision3_quest_module(quest, wrong_layer),
        Err(Revision3QuestGenerationError::CollisionCatalogLayerMismatch)
    ));
}

#[test]
fn missing_or_mismatched_raw_artifact_asset_is_rejected() {
    let project = empty_basis();
    let mut missing = project.clone();
    missing.asset_store.assets.clear();
    assert!(matches!(
        rejected(evaluate(
            &missing,
            &request(&missing),
            collision_input(&missing)
        )),
        Revision3QuestDraftInsertConflictV2::MissingArtifactAsset { .. }
    ));

    let mut metadata = project;
    let digest = artifact_reference(&metadata).artifact.sha256;
    metadata
        .asset_store
        .assets
        .get_mut(&digest)
        .unwrap()
        .byte_len += 1;
    assert!(matches!(
        rejected(evaluate(
            &metadata,
            &request(&metadata),
            collision_input(&metadata)
        )),
        Revision3QuestDraftInsertConflictV2::ArtifactAssetMetadataMismatch { .. }
    ));

    let mut media_type = empty_basis();
    let digest = artifact_reference(&media_type).artifact.sha256;
    media_type
        .asset_store
        .assets
        .get_mut(&digest)
        .unwrap()
        .media_type = "application/json".to_owned();
    assert!(matches!(
        rejected(evaluate(
            &media_type,
            &request(&media_type),
            collision_input(&media_type)
        )),
        Revision3QuestDraftInsertConflictV2::ArtifactAssetMetadataMismatch { .. }
    ));
}

#[test]
fn recursive_and_residual_quest_basis_are_never_extended() {
    let basis = empty_basis();
    let first = applied(evaluate(&basis, &request(&basis), collision_input(&basis)));
    let recursive = first.project;
    let second_request = Revision3QuestDraftInsertRequestV2 {
        expected_revision: recursive.revision,
        quest_id: entity_id(0x61),
        script_module_id: entity_id(0x62),
        ..request(&recursive)
    };
    assert!(matches!(
        rejected(evaluate(
            &recursive,
            &second_request,
            collision_input(&recursive)
        )),
        Revision3QuestDraftInsertConflictV2::RecursiveQuestBasis { .. }
    ));

    let mut residual = recursive;
    residual.entities.remove(&entity_id(0x51));
    let residual_request = Revision3QuestDraftInsertRequestV2 {
        expected_revision: residual.revision,
        quest_id: entity_id(0x63),
        script_module_id: entity_id(0x64),
        ..request(&residual)
    };
    assert!(matches!(
        rejected(evaluate(
            &residual,
            &residual_request,
            collision_input(&residual)
        )),
        Revision3QuestDraftInsertConflictV2::ResidualQuestBasis { .. }
    ));
}

#[test]
fn revision_overflow_and_invalid_closed_parent_return_no_candidate() {
    let mut project = empty_basis();
    project.revision = u64::MAX;
    assert!(matches!(
        rejected(evaluate(
            &project,
            &request(&project),
            collision_input(&project)
        )),
        Revision3QuestDraftInsertConflictV2::ProjectRevisionOverflow
    ));

    let project = empty_basis();
    let mut invalid_parent = request(&project);
    invalid_parent.intent.parent_quest.runtime_class = "NotAQuestClass".to_owned();
    assert!(matches!(
        rejected(evaluate(
            &project,
            &invalid_parent,
            collision_input(&project)
        )),
        Revision3QuestDraftInsertConflictV2::InvalidQuestIntent {
            error: DraftQuestSkeletonError::InvalidParentQuestClass
        }
    ));
}

#[test]
fn non_revision3_input_is_rejected() {
    let project = empty_basis();
    let revision3 = project.to_canonical_json().unwrap();
    let unsupported = revision3.replacen("\"schema_revision\":3", "\"schema_revision\":2", 1);
    let request = request(&project).to_canonical_json().unwrap();
    assert!(matches!(
        apply_revision3_quest_draft_transaction_v2(
            &unsupported,
            &request,
            collision_input(&project)
        ),
        Err(Revision3QuestDraftInsertErrorV2::InvalidProject(_))
    ));
}

fn npc_parent(
    project: &ProjectRevision3,
    seal_value: u8,
    selector: &str,
    runtime_class: &str,
) -> NpcParentClassInput {
    NpcParentClassInput {
        generation: project.target.clone(),
        source_seal: seal(seal_value, 10_000),
        catalog_layer: "base-game.g1r.characters".to_owned(),
        canonical_selector: selector.to_owned(),
        runtime_class: runtime_class.to_owned(),
    }
}

fn add_valid_npc(project: &mut ProjectRevision3, runtime_id: &str, module_namespace: &str) {
    let npc_id = entity_id(0x31);
    let module_id = entity_id(0x32);
    let owner = Revision3TypedRef::new(project.project_id, npc_id, Revision3EntityKind::NpcDraft);
    let draft = Revision3NpcDraft {
        generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
        generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
        input: Revision3NpcDraftInput {
            target: project.target.clone(),
            module_namespace: module_namespace.to_owned(),
            unique_name: runtime_id.to_owned(),
            parent_character_definition: npc_parent(
                project,
                0x31,
                "CatalogCharacterDefinition_Base",
                "UCharacterDefinition_Human_Base",
            ),
            parent_ai_agent_config: npc_parent(
                project,
                0x32,
                "CatalogAiAgentConfig_Base",
                "UAIAgentConfig_Human_Base",
            ),
            parent_spawn_definition: npc_parent(
                project,
                0x33,
                "CatalogSpawnDefinition_Base",
                "USpawnAIAgentDefinition_Base",
            ),
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
            display_name: "Existing NPC".to_owned(),
            origin: Revision3OriginRef::New {
                authored_runtime_id: runtime_id.to_owned(),
            },
            revision: 0,
            payload: Revision3EntityPayload::NpcDraft(draft),
        },
    );
    project.entities.insert(
        module_id,
        Revision3Entity {
            id: module_id,
            display_name: "Existing NPC source".to_owned(),
            origin: Revision3OriginRef::Generated {
                generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
                generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
                owner,
            },
            revision: 0,
            payload: Revision3EntityPayload::ScriptModule(module),
        },
    );
}

#[test]
fn exact_basis_runtime_and_entity_collisions_are_checked_independently() {
    let mut project = empty_basis();
    add_valid_npc(
        &mut project,
        "GORE_S4_ASGHAN_TRIAL",
        "GoreMods.Npcs.Existing",
    );
    let base_request = request(&project);
    assert!(matches!(
        rejected(evaluate(
            &project,
            &base_request,
            collision_input(&project)
        )),
        Revision3QuestDraftInsertConflictV2::StoryIdentityCollision {
            kind: Revision3StoryIdentityKindV2::AuthoredRuntimeId,
            existing_entity: Some(existing),
            ..
        } if existing == entity_id(0x31)
    ));

    let mut id_collision = base_request;
    id_collision.quest_id = entity_id(0x31);
    assert!(matches!(
        rejected(evaluate(
            &project,
            &id_collision,
            collision_input(&project)
        )),
        Revision3QuestDraftInsertConflictV2::EntityIdCollision {
            role: Revision3QuestEntityRoleV2::QuestDraft,
            entity,
        } if entity == entity_id(0x31)
    ));

    let mut module_project = empty_basis();
    add_valid_npc(
        &mut module_project,
        "ExistingNpc",
        "GoreMods.Quests.S4AsghanTrial",
    );
    assert!(matches!(
        rejected(evaluate(
            &module_project,
            &request(&module_project),
            collision_input(&module_project)
        )),
        Revision3QuestDraftInsertConflictV2::StoryIdentityCollision {
            kind: Revision3StoryIdentityKindV2::ModuleNamespace,
            existing_entity: Some(existing),
            ..
        } if existing == entity_id(0x31)
    ));

    let mut symbol_project = empty_basis();
    add_valid_npc(&mut symbol_project, "ExistingNpc", "GoreMods.Npcs.Existing");
    let mut symbol_request = request(&symbol_project);
    symbol_request.intent.text_helper = "UCharacterDefinition_Human_ExistingNpc".to_owned();
    assert!(matches!(
        rejected(evaluate(
            &symbol_project,
            &symbol_request,
            collision_input(&symbol_project)
        )),
        Revision3QuestDraftInsertConflictV2::StoryIdentityCollision {
            kind: Revision3StoryIdentityKindV2::GeneratedSymbol,
            existing_entity: Some(existing),
            ..
        } if existing == entity_id(0x31)
    ));
}
