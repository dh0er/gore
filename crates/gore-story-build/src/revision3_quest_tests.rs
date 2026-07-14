use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::{
    migrate_revision2_to_revision3, AssetStoreIndex, ContentSeal, DraftQuestSkeletonError,
    EntityId, FormatV2, GameGenerationAnchor, OpenedRevision3Checkpoint, ProjectId, ProjectMeta,
    ProjectRevision3, QuestCollisionArtifactRef, QuestCollisionCatalogInput, Revision3Entity,
    Revision3EntityKind, Revision3EntityPayload, Revision3OriginRef, Revision3QuestDraft,
    Revision3QuestDraftInput, Revision3QuestGenerationError, Revision3QuestGiverInput,
    Revision3QuestParentInput, Revision3ScriptModule, Revision3TypedRef, SchemaRevisionV3,
    Sha256Digest, WorkingHead, WorkingStoreFormat, QUEST_COLLISION_CATALOG_LAYER,
    REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION, REVISION3_QUEST_GENERATOR_ID,
    REVISION3_QUEST_GENERATOR_VERSION,
};
use gore_story_inventory::{QuestCollisionCapabilityArtifactV1, VerifiedQuestCollisionCapability};
use sha2::{Digest as _, Sha256};

use crate::revision3_quest::{
    prepare_revision3_quest_source_inspection, project_revision3_basis_to_revision2,
    regenerate_revision3_quest_module, revision3_quest_input_fingerprint, PlanFormat,
    PlanSchemaRevision, PreparedRevision3QuestSourceInspection, QuestInspectionBuildStatus,
    QuestInspectionPublicationStatus, QuestInspectionRuntimeQualification, QuestInspectionScope,
    Revision3QuestInspectionError, Revision3QuestInspectionModule,
    Revision3QuestInspectionProvenance, Revision3QuestSourceInspectionPlanV2,
};

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

fn seal_bytes(bytes: &[u8]) -> ContentSeal {
    ContentSeal {
        byte_len: bytes.len() as u64,
        sha256: Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
    }
}

fn target() -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: seal(1, 171_698_176),
    }
}

fn parent() -> Revision3QuestParentInput {
    Revision3QuestParentInput {
        generation: target(),
        source_seal: seal(2, 856),
        catalog_layer: "base-game.g1r.scripts".to_owned(),
        canonical_selector: "CatalogQuestParent".to_owned(),
        runtime_class: "UQuest_SwampCamp_SCCHAPTER2".to_owned(),
    }
}

fn giver() -> Revision3QuestGiverInput {
    Revision3QuestGiverInput {
        generation: target(),
        source_seal: seal(3, 856),
        catalog_layer: "base-game.g1r.scripts".to_owned(),
        canonical_selector: "CatalogAsghan".to_owned(),
        runtime_unique_name: "OM_GRD_Asghan_263".to_owned(),
    }
}

fn artifact_ref() -> QuestCollisionArtifactRef {
    QuestCollisionArtifactRef {
        generation: target(),
        catalog_layer: QUEST_COLLISION_CATALOG_LAYER.to_owned(),
        artifact: seal(4, 4_096),
        source_seal: seal(5, 4_096),
        basis_snapshot: seal(6, 800),
    }
}

fn quest(project: ProjectId) -> Revision3QuestDraft {
    Revision3QuestDraft {
        generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
        generator_version: REVISION3_QUEST_GENERATOR_VERSION,
        input: Revision3QuestDraftInput {
            target: target(),
            quest_id: entity_id(10),
            module_namespace: "GoreMods.Quests.AsghanTrial".to_owned(),
            technical_id: "GORE_ASGHAN_TRIAL".to_owned(),
            text_helper: "GoreQuestText".to_owned(),
            parent_quest: parent(),
            giver: giver(),
            title: "Asghan Trial".to_owned(),
            description: "Prove that the gate is secure.".to_owned(),
            objective_title: "Report to Asghan".to_owned(),
            additional_objective_titles: Vec::new(),
            transition_plan: None,
            collision_catalog: artifact_ref(),
        },
        script_module: Revision3TypedRef::new(
            project,
            entity_id(11),
            Revision3EntityKind::ScriptModule,
        ),
    }
}

fn collision_input() -> QuestCollisionCatalogInput {
    QuestCollisionCatalogInput {
        generation: target(),
        source_seal: artifact_ref().source_seal,
        catalog_layer: QUEST_COLLISION_CATALOG_LAYER.to_owned(),
        modules: BTreeSet::new(),
        relative_paths: BTreeSet::new(),
        symbols: BTreeSet::new(),
    }
}

fn empty_basis() -> ProjectRevision3 {
    ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id: project_id(8),
        revision: 3,
        meta: ProjectMeta {
            name: "Quest basis".to_owned(),
            version: "0.1.0".to_owned(),
            author: "test".to_owned(),
        },
        target: target(),
        authoring_locales: BTreeSet::new(),
        entities: BTreeMap::new(),
        asset_store: AssetStoreIndex::default(),
    }
}

fn opened_basis(project: ProjectRevision3) -> OpenedRevision3Checkpoint {
    OpenedRevision3Checkpoint {
        head: WorkingHead {
            store_format: WorkingStoreFormat,
            snapshot: artifact_ref().basis_snapshot,
        },
        project,
    }
}

fn quest_entity(draft: Revision3QuestDraft) -> Revision3Entity {
    Revision3Entity {
        id: draft.input.quest_id,
        display_name: "Asghan Trial".to_owned(),
        origin: Revision3OriginRef::New {
            authored_runtime_id: draft.input.technical_id.clone(),
        },
        revision: 0,
        payload: Revision3EntityPayload::QuestDraft(draft),
    }
}

fn module_entity(project: ProjectId, module: Revision3ScriptModule) -> Revision3Entity {
    Revision3Entity {
        id: entity_id(11),
        display_name: "Asghan Trial source".to_owned(),
        origin: Revision3OriginRef::Generated {
            generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
            generator_version: REVISION3_QUEST_GENERATOR_VERSION,
            owner: Revision3TypedRef::new(project, entity_id(10), Revision3EntityKind::QuestDraft),
        },
        revision: 0,
        payload: Revision3EntityPayload::ScriptModule(module),
    }
}

#[test]
fn v2_fingerprint_binds_raw_semantic_and_basis_seals() {
    let input = quest(project_id(8)).input;
    let baseline = revision3_quest_input_fingerprint(&input).unwrap();
    let edits: [fn(&mut Revision3QuestDraftInput); 3] = [
        |value| {
            value.collision_catalog.artifact.sha256 = Sha256Digest::from_bytes([0x41; 32]);
        },
        |value| {
            value.collision_catalog.source_seal.sha256 = Sha256Digest::from_bytes([0x42; 32]);
        },
        |value| {
            value.collision_catalog.basis_snapshot.sha256 = Sha256Digest::from_bytes([0x43; 32]);
        },
    ];
    for edit in edits {
        let mut changed = input.clone();
        edit(&mut changed);
        assert_ne!(
            revision3_quest_input_fingerprint(&changed).unwrap(),
            baseline
        );
    }
}

#[test]
fn v2_lowering_is_deterministic_and_checks_the_moved_collision_set() {
    let draft = quest(project_id(8));
    let first = regenerate_revision3_quest_module(&draft, collision_input()).unwrap();
    let second = regenerate_revision3_quest_module(&draft, collision_input()).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.generator_version, REVISION3_QUEST_GENERATOR_VERSION);
    assert_eq!(
        first.input_fingerprint,
        revision3_quest_input_fingerprint(&draft.input).unwrap()
    );
    assert!(first.source.contains("class UQuest_GORE_ASGHAN_TRIAL"));

    let mut collision = collision_input();
    collision
        .modules
        .insert("goremods.quests.asghantrial".to_owned());
    assert!(matches!(
        regenerate_revision3_quest_module(&draft, collision),
        Err(Revision3QuestInspectionError::InvalidQuestIntent(
            DraftQuestSkeletonError::GeneratedNameCollision { .. }
        ))
    ));
}

#[test]
fn v3_multi_objective_lowering_preserves_order_and_reserves_every_symbol() {
    let mut draft = quest(project_id(8));
    draft.generator_version = REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION;
    draft.input.additional_objective_titles = vec![
        "Inspect the gate".to_owned(),
        "Report the secured gate".to_owned(),
    ];
    let generated = regenerate_revision3_quest_module(&draft, collision_input()).unwrap();
    assert_eq!(
        generated.generator_version,
        REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION
    );
    let first = generated.source.find("_OBJ_DONE").unwrap();
    let second = generated.source.find("_OBJ_2").unwrap();
    let third = generated.source.find("_OBJ_3").unwrap();
    assert!(first < second && second < third);
    assert_eq!(generated.source.matches("bSucceedParent = true").count(), 1);

    for symbol in [
        "UQuest_GORE_ASGHAN_TRIAL_OBJ_2",
        "GetGoreAsghanTrialObjective3",
    ] {
        let mut collision = collision_input();
        collision.symbols.insert(symbol.to_owned());
        assert!(matches!(
            regenerate_revision3_quest_module(&draft, collision),
            Err(Revision3QuestInspectionError::InvalidQuestIntent(
                DraftQuestSkeletonError::GeneratedNameCollision {
                    kind: gore_authoring::DraftQuestCollisionKind::Symbol,
                    ..
                }
            ))
        ));
    }

    let mut wrong_version = draft.clone();
    wrong_version.generator_version = REVISION3_QUEST_GENERATOR_VERSION;
    assert!(matches!(
        regenerate_revision3_quest_module(&wrong_version, collision_input()),
        Err(Revision3QuestInspectionError::SharedQuestGeneration(
            Revision3QuestGenerationError::ObjectiveGeneratorContract { .. }
        ))
    ));
}

#[test]
fn s3_lowering_propagates_all_shared_collision_reference_mismatches() {
    let draft = quest(project_id(8));

    let mut wrong_generation = collision_input();
    wrong_generation.generation = GameGenerationAnchor {
        executable: seal(0x71, 171_698_176),
    };
    assert!(matches!(
        regenerate_revision3_quest_module(&draft, wrong_generation),
        Err(Revision3QuestInspectionError::SharedQuestGeneration(
            Revision3QuestGenerationError::CollisionGenerationMismatch
        ))
    ));

    let mut wrong_source = collision_input();
    wrong_source.source_seal.sha256 = Sha256Digest::from_bytes([0x72; 32]);
    assert!(matches!(
        regenerate_revision3_quest_module(&draft, wrong_source),
        Err(Revision3QuestInspectionError::SharedQuestGeneration(
            Revision3QuestGenerationError::CollisionSourceSealMismatch
        ))
    ));

    let mut wrong_layer = collision_input();
    wrong_layer.catalog_layer = "foreign.story-collisions.v1".to_owned();
    assert!(matches!(
        regenerate_revision3_quest_module(&draft, wrong_layer),
        Err(Revision3QuestInspectionError::SharedQuestGeneration(
            Revision3QuestGenerationError::CollisionCatalogLayerMismatch
        ))
    ));
}

#[test]
fn basis_projection_roundtrips_and_rejects_recursive_or_residual_quest_state() {
    let clean = empty_basis();
    let lowered = project_revision3_basis_to_revision2(&opened_basis(clean.clone())).unwrap();
    assert_eq!(
        migrate_revision2_to_revision3(&lowered).unwrap().project,
        clean
    );

    let mut recursive = empty_basis();
    let draft = quest(recursive.project_id);
    recursive
        .entities
        .insert(draft.input.quest_id, quest_entity(draft));
    assert!(matches!(
        project_revision3_basis_to_revision2(&opened_basis(recursive)),
        Err(Revision3QuestInspectionError::RecursiveQuestBasis { .. })
    ));

    let mut residual = empty_basis();
    let draft = quest(residual.project_id);
    let module = regenerate_revision3_quest_module(&draft, collision_input()).unwrap();
    residual
        .entities
        .insert(entity_id(11), module_entity(residual.project_id, module));
    assert!(matches!(
        project_revision3_basis_to_revision2(&opened_basis(residual)),
        Err(Revision3QuestInspectionError::ResidualQuestBasis { .. })
    ));
}

#[test]
fn plan_reopen_is_canonical_bounded_and_permanently_fail_closed() {
    let project = project_id(8);
    let draft = quest(project);
    let generated = regenerate_revision3_quest_module(&draft, collision_input()).unwrap();
    let input = serde_json::to_vec(&draft.input).unwrap();
    let source = seal_bytes(generated.source.as_bytes());
    let plan = Revision3QuestSourceInspectionPlanV2 {
        format_marker: PlanFormat,
        schema_revision: PlanSchemaRevision,
        scope: QuestInspectionScope::SourceInspectionOnly,
        build_status: QuestInspectionBuildStatus::Blocked,
        runtime_qualification: QuestInspectionRuntimeQualification::RuntimeUnqualified,
        publication_status: QuestInspectionPublicationStatus::NotSupported,
        provenance: Revision3QuestInspectionProvenance {
            project_id: project,
            project_revision: 4,
            target_executable: target().executable,
            canonical_project: seal(20, 1_000),
            basis_snapshot: artifact_ref().basis_snapshot,
            canonical_collision_source_project: seal(21, 900),
            collision_artifact: artifact_ref().artifact,
            collision_source: artifact_ref().source_seal,
        },
        module: Revision3QuestInspectionModule {
            quest: Revision3TypedRef::new(project, entity_id(10), Revision3EntityKind::QuestDraft),
            script_module: draft.script_module,
            draft_input: seal_bytes(&input),
            persisted_source: source,
            generated,
        },
    };
    let canonical = plan.to_canonical_json().unwrap();
    assert_eq!(
        Revision3QuestSourceInspectionPlanV2::from_json(&canonical).unwrap(),
        plan
    );
    assert!(matches!(
        Revision3QuestSourceInspectionPlanV2::from_json(&(canonical.clone() + "\n")),
        Err(Revision3QuestInspectionError::NonCanonicalPlanJson)
    ));
    assert!(matches!(
        Revision3QuestSourceInspectionPlanV2::from_json(
            &canonical.replace("\"blocked\"", "\"ready\"")
        ),
        Err(Revision3QuestInspectionError::InvalidPlanJson(_))
    ));
    let mut oversized = plan;
    oversized.module.draft_input.byte_len = u64::MAX;
    assert!(matches!(
        oversized.to_canonical_json(),
        Err(Revision3QuestInspectionError::PlanInvariant(_))
    ));
}

#[test]
fn public_lowering_boundary_accepts_only_a_fresh_capability_not_an_artifact() {
    let _: fn(
        PreparedRevision3QuestSourceInspection,
        VerifiedQuestCollisionCapability,
    ) -> Result<Revision3QuestSourceInspectionPlanV2, Revision3QuestInspectionError> =
        PreparedRevision3QuestSourceInspection::lower;
    let _: fn(
        &gore_authoring::WorkingProjectStore,
        &str,
        EntityId,
    )
        -> Result<PreparedRevision3QuestSourceInspection, Revision3QuestInspectionError> =
        prepare_revision3_quest_source_inspection;
    let _: Option<QuestCollisionCapabilityArtifactV1> = None;
}
