use std::collections::BTreeSet;

use gore_authoring::{
    ContentSeal, DraftQuestSkeletonError, EntityId, GameGenerationAnchor, ProjectId,
    QuestCollisionArtifactRef, QuestCollisionCatalogInput, Revision3EntityKind,
    Revision3QuestDraft, Revision3QuestDraftInput, Revision3QuestGenerationError,
    Revision3QuestGiverInput, Revision3QuestParentInput, Revision3TypedRef, Sha256Digest,
    WorkingHead, WorkingStoreFormat, QUEST_COLLISION_CATALOG_LAYER_V2,
    REVISION3_QUEST_GENERATOR_ID, REVISION3_QUEST_GENERATOR_VERSION,
};
use gore_story_inventory::VerifiedRevision3QuestCollisionInspectionCapabilityV2;
use sha2::{Digest as _, Sha256};

use crate::revision3_quest::{
    prepare_revision3_quest_source_inspection_v3, regenerate_revision3_quest_module,
    revision3_quest_input_fingerprint, PlanFormat, PlanSchemaRevisionV3,
    PreparedRevision3QuestSourceInspectionV3, QuestInspectionBuildStatus,
    QuestInspectionPublicationStatus, QuestInspectionRuntimeQualification, QuestInspectionScope,
    Revision3QuestInspectionError, Revision3QuestInspectionModule,
    Revision3QuestInspectionProvenanceV3, Revision3QuestSourceInspectionPlanV3,
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
        catalog_layer: QUEST_COLLISION_CATALOG_LAYER_V2.to_owned(),
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
            transition_plan: Box::new(
                gore_authoring::QuestTransitionPlanV1::default_for_objectives(1).unwrap(),
            ),
            collision_catalog: artifact_ref(),
        },
        script_module: Revision3TypedRef::new(
            project,
            entity_id(11),
            Revision3EntityKind::ScriptModule,
        ),
        transcript: Vec::new(),
    }
}

fn collision_input() -> QuestCollisionCatalogInput {
    QuestCollisionCatalogInput {
        generation: target(),
        source_seal: artifact_ref().source_seal,
        catalog_layer: QUEST_COLLISION_CATALOG_LAYER_V2.to_owned(),
        modules: BTreeSet::new(),
        relative_paths: BTreeSet::new(),
        symbols: BTreeSet::new(),
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
fn current_multi_objective_lowering_preserves_order_and_reserves_every_symbol() {
    let mut draft = quest(project_id(8));
    draft.input.additional_objective_titles = vec![
        "Inspect the gate".to_owned(),
        "Report the secured gate".to_owned(),
    ];
    draft.input.transition_plan =
        Box::new(gore_authoring::QuestTransitionPlanV1::default_for_objectives(3).unwrap());
    let generated = regenerate_revision3_quest_module(&draft, collision_input()).unwrap();
    assert_eq!(
        generated.generator_version,
        REVISION3_QUEST_GENERATOR_VERSION
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
    wrong_version.generator_version = REVISION3_QUEST_GENERATOR_VERSION - 1;
    assert!(matches!(
        regenerate_revision3_quest_module(&wrong_version, collision_input()),
        Err(Revision3QuestInspectionError::SharedQuestGeneration(
            Revision3QuestGenerationError::GeneratorContract { .. }
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
fn current_plan_reopen_is_canonical_bounded_and_permanently_fail_closed() {
    let project = project_id(8);
    let draft = quest(project);
    let generated = regenerate_revision3_quest_module(&draft, collision_input()).unwrap();
    let input = serde_json::to_vec(&draft.input).unwrap();
    let source = seal_bytes(generated.source.as_bytes());
    let plan = Revision3QuestSourceInspectionPlanV3 {
        format_marker: PlanFormat,
        schema_revision: PlanSchemaRevisionV3,
        scope: QuestInspectionScope::SourceInspectionOnly,
        build_status: QuestInspectionBuildStatus::Blocked,
        runtime_qualification: QuestInspectionRuntimeQualification::RuntimeUnqualified,
        publication_status: QuestInspectionPublicationStatus::NotSupported,
        provenance: Revision3QuestInspectionProvenanceV3 {
            project_id: project,
            project_revision: 4,
            target_executable: target().executable,
            canonical_project: seal(20, 1_000),
            collision_basis_head: WorkingHead {
                store_format: WorkingStoreFormat,
                snapshot: artifact_ref().basis_snapshot,
            },
            collision_basis_project: seal(21, 900),
            collision_nonquest_project: seal(22, 700),
            collision_prior_quest_count: 0,
            collision_prior_quest_evidence: seal(23, 600),
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
        Revision3QuestSourceInspectionPlanV3::from_json(&canonical).unwrap(),
        plan
    );
    assert!(matches!(
        Revision3QuestSourceInspectionPlanV3::from_json(&(canonical.clone() + "\n")),
        Err(Revision3QuestInspectionError::NonCanonicalPlanJson)
    ));
    assert!(matches!(
        Revision3QuestSourceInspectionPlanV3::from_json(
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
        PreparedRevision3QuestSourceInspectionV3,
        VerifiedRevision3QuestCollisionInspectionCapabilityV2,
    ) -> Result<Revision3QuestSourceInspectionPlanV3, Revision3QuestInspectionError> =
        PreparedRevision3QuestSourceInspectionV3::lower;
    let _: fn(
        &gore_authoring::WorkingProjectStore,
        &str,
        EntityId,
    )
        -> Result<PreparedRevision3QuestSourceInspectionV3, Revision3QuestInspectionError> =
        prepare_revision3_quest_source_inspection_v3;
}
