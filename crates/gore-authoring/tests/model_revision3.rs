use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::{
    AssetMeta, AssetStoreIndex, ContentSeal, EntityId, FormatV2, GameGenerationAnchor, ProjectId,
    ProjectMeta, ProjectRevision3, ProjectRevision3JsonError, ProjectRevision3ValidationError,
    QuestCollisionArtifactRef, QuestGiverInput, QuestParentInput, QuestTransitionPlanV1,
    Revision3Entity, Revision3EntityKind, Revision3EntityPayload, Revision3OriginRef,
    Revision3QuestDraft, Revision3QuestDraftInput, Revision3ScriptModule, Revision3TypedRef,
    SchemaRevisionV3, ScriptModuleStatus, Sha256Digest, MAX_PROJECT_JSON_BYTES,
    MAX_QUEST_COLLISION_ARTIFACT_BYTES, MAX_REVISION3_ENTITY_JSON_BYTES,
    QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2, QUEST_COLLISION_CATALOG_LAYER_V2,
    REVISION3_QUEST_GENERATOR_ID, REVISION3_QUEST_GENERATOR_VERSION,
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

fn target() -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: seal(1, 171_698_176),
    }
}

fn empty_revision3() -> ProjectRevision3 {
    ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id: project_id(1),
        revision: 7,
        meta: ProjectMeta {
            name: "Revision 3".into(),
            version: "0.1.0".into(),
            author: "tests".into(),
        },
        target: target(),
        authoring_locales: BTreeSet::new(),
        entities: BTreeMap::new(),
        asset_store: AssetStoreIndex::default(),
    }
}

fn parent() -> QuestParentInput {
    QuestParentInput {
        generation: target(),
        source_seal: seal(2, 20_000),
        catalog_layer: "base-game.g1r.quests".into(),
        canonical_selector: "CatalogQuest_Parent".into(),
        runtime_class: "UQuest_Parent".into(),
    }
}

fn giver() -> QuestGiverInput {
    QuestGiverInput {
        generation: target(),
        source_seal: seal(3, 30_000),
        catalog_layer: "base-game.g1r.characters".into(),
        canonical_selector: "CatalogCharacter_Asghan".into(),
        runtime_unique_name: "OM_GRD_Asghan_263".into(),
    }
}

fn quest_project() -> ProjectRevision3 {
    let mut project = empty_revision3();
    let quest_id = entity_id(10);
    let module_id = entity_id(11);
    let artifact = seal(4, 3_517_569);
    project.asset_store.assets.insert(
        artifact.sha256,
        AssetMeta {
            byte_len: artifact.byte_len,
            media_type: QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2.into(),
        },
    );
    let quest = Revision3QuestDraft {
        generator_id: REVISION3_QUEST_GENERATOR_ID.into(),
        generator_version: REVISION3_QUEST_GENERATOR_VERSION,
        input: Revision3QuestDraftInput {
            target: target(),
            quest_id,
            module_namespace: "GoreMods.Quests.AsghanTrial".into(),
            technical_id: "GORE_ASGHAN_TRIAL".into(),
            text_helper: "GoreQuestText".into(),
            parent_quest: parent(),
            giver: giver(),
            title: "Asghan's Trial".into(),
            description: "Prove that the gate is secure.".into(),
            objective_title: "Report to Asghan".into(),
            additional_objective_titles: Vec::new(),
            transition_plan: Box::new(QuestTransitionPlanV1::default_for_objectives(1).unwrap()),
            collision_catalog: QuestCollisionArtifactRef {
                generation: target(),
                catalog_layer: QUEST_COLLISION_CATALOG_LAYER_V2.into(),
                artifact: artifact.clone(),
                source_seal: seal(5, artifact.byte_len),
                basis_snapshot: seal(6, 800),
            },
        },
        script_module: Revision3TypedRef::new(
            project.project_id,
            module_id,
            Revision3EntityKind::ScriptModule,
        ),
        transcript: Vec::new(),
    };
    let source = "// resolver-bound revision-3 Quest source\n".to_owned();
    let module = Revision3ScriptModule {
        generator_id: REVISION3_QUEST_GENERATOR_ID.into(),
        generator_version: REVISION3_QUEST_GENERATOR_VERSION,
        owner: Revision3TypedRef::new(
            project.project_id,
            quest_id,
            Revision3EntityKind::QuestDraft,
        ),
        module_namespace: quest.input.module_namespace.clone(),
        module_relative_path: "GoreMods/Quests/AsghanTrial.as".into(),
        source_sha256: Sha256Digest::from_bytes(Sha256::digest(source.as_bytes()).into()),
        source,
        input_fingerprint: Sha256Digest::from_bytes([7; 32]),
        status: ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED,
    };
    project.entities.insert(
        quest_id,
        Revision3Entity {
            id: quest_id,
            display_name: "Quest Asghan Trial".into(),
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
            display_name: "Asghan Trial source".into(),
            origin: Revision3OriginRef::Generated {
                generator_id: REVISION3_QUEST_GENERATOR_ID.into(),
                generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                owner: Revision3TypedRef::new(
                    project.project_id,
                    quest_id,
                    Revision3EntityKind::QuestDraft,
                ),
            },
            revision: 0,
            payload: Revision3EntityPayload::ScriptModule(module),
        },
    );
    project
}

#[test]
fn revision3_is_exact_canonical_duplicate_safe_and_standalone() {
    let project = quest_project();
    let canonical = project.to_canonical_json().unwrap();
    assert_eq!(ProjectRevision3::from_json(&canonical).unwrap(), project);
    assert!(canonical.contains("\"schema_revision\":3"));
    assert!(!canonical.contains("\"modules\""));
    assert!(!canonical.contains("\"relative_paths\""));
    assert!(!canonical.contains("\"symbols\""));
    assert!(!canonical.contains("\"additional_objective_titles\""));
    assert!(canonical.contains("\"transition_plan\""));

    let whitespace = format!(" {canonical}");
    assert!(matches!(
        ProjectRevision3::from_json(&whitespace),
        Err(ProjectRevision3JsonError::NonCanonicalJson)
    ));
    let duplicate = canonical.replacen("\"revision\":7", "\"revision\":7,\"revision\":7", 1);
    assert!(matches!(
        ProjectRevision3::from_json(&duplicate),
        Err(ProjectRevision3JsonError::InvalidJson(_))
    ));
    let unknown = canonical.replacen("\"revision\":7", "\"unknown\":0,\"revision\":7", 1);
    assert!(matches!(
        ProjectRevision3::from_json(&unknown),
        Err(ProjectRevision3JsonError::InvalidJson(_))
    ));

    let invalid = canonical.replacen(
        &format!("\"project_id\":\"{}\"", project.project_id),
        &format!("\"project_id\":\"{}\"", ProjectId::from_bytes([0; 16])),
        1,
    );
    let error = ProjectRevision3::from_json(&invalid).unwrap_err();
    assert!(
        matches!(error, ProjectRevision3JsonError::InvalidJson(_)),
        "unexpected zero-project-id error: {error:?}"
    );
}

#[test]
fn revision3_multi_objectives_round_trip_in_order_with_stable_slots() {
    let mut project = quest_project();
    let quest_id = entity_id(10);
    let module_id = entity_id(11);
    let Revision3EntityPayload::QuestDraft(quest) =
        &mut project.entities.get_mut(&quest_id).unwrap().payload
    else {
        panic!("fixture Quest missing")
    };
    quest.generator_version = REVISION3_QUEST_GENERATOR_VERSION;
    quest.input.additional_objective_titles =
        vec!["Inspect the gate".into(), "Report the secured mine".into()];
    quest.input.transition_plan =
        Box::new(QuestTransitionPlanV1::default_for_objectives(3).unwrap());
    let Revision3EntityPayload::ScriptModule(module) =
        &mut project.entities.get_mut(&module_id).unwrap().payload
    else {
        panic!("fixture module missing")
    };
    module.generator_version = REVISION3_QUEST_GENERATOR_VERSION;
    let Revision3OriginRef::Generated {
        generator_version, ..
    } = &mut project.entities.get_mut(&module_id).unwrap().origin
    else {
        panic!("fixture module origin missing")
    };
    *generator_version = REVISION3_QUEST_GENERATOR_VERSION;

    project.validate_closed_model().unwrap();
    let canonical = project.to_canonical_json().unwrap();
    assert!(canonical.contains(
        "\"objective_title\":\"Report to Asghan\",\"additional_objective_titles\":[\"Inspect the gate\",\"Report the secured mine\"]"
    ));
    assert_eq!(ProjectRevision3::from_json(&canonical).unwrap(), project);

    let Revision3EntityPayload::QuestDraft(quest) =
        &mut project.entities.get_mut(&quest_id).unwrap().payload
    else {
        unreachable!()
    };
    quest.input.transition_plan =
        Box::new(QuestTransitionPlanV1::default_for_objectives(1).unwrap());
    assert!(matches!(
        project.validate_closed_model(),
        Err(ProjectRevision3ValidationError::InvalidQuestArtifactRef { .. })
    ));
}

#[test]
fn quest_wire_requires_a_transition_plan_and_generator_v4() {
    let mut project = quest_project();
    let quest_id = entity_id(10);
    let Revision3EntityPayload::QuestDraft(quest) =
        &mut project.entities.get_mut(&quest_id).unwrap().payload
    else {
        panic!("fixture Quest missing")
    };
    let mut input = serde_json::to_value(&quest.input).unwrap();
    input.as_object_mut().unwrap().remove("transition_plan");
    assert!(serde_json::from_value::<Revision3QuestDraftInput>(input).is_err());

    quest.generator_version = 3;
    assert!(matches!(
        project.validate_closed_model(),
        Err(ProjectRevision3ValidationError::InvalidQuestArtifactRef { .. })
    ));
}

#[test]
fn collision_arrays_and_cross_revision_markers_are_not_revision3_wire() {
    let canonical = quest_project().to_canonical_json().unwrap();
    let with_array = canonical.replacen(
        "\"basis_snapshot\":",
        "\"modules\":[],\"basis_snapshot\":",
        1,
    );
    assert!(matches!(
        ProjectRevision3::from_json(&with_array),
        Err(ProjectRevision3JsonError::InvalidJson(_))
    ));

    let unsupported_schema =
        canonical.replacen("\"schema_revision\":3", "\"schema_revision\":2", 1);
    assert!(matches!(
        ProjectRevision3::from_json(&unsupported_schema),
        Err(ProjectRevision3JsonError::InvalidJson(_))
    ));
}

#[test]
fn artifact_reference_target_layer_lengths_and_asset_index_fail_closed() {
    let base = quest_project();
    let quest_id = entity_id(10);
    let artifact_digest = seal(4, 3_517_569).sha256;

    let mutate_ref = |project: &mut ProjectRevision3,
                      edit: &dyn Fn(&mut QuestCollisionArtifactRef)| {
        let Revision3EntityPayload::QuestDraft(quest) =
            &mut project.entities.get_mut(&quest_id).unwrap().payload
        else {
            panic!("expected Quest")
        };
        edit(&mut quest.input.collision_catalog);
    };

    let mut wrong_generation = base.clone();
    mutate_ref(&mut wrong_generation, &|reference| {
        reference.generation.executable.sha256 = Sha256Digest::from_bytes([0x91; 32]);
    });
    assert!(matches!(
        wrong_generation.validate_closed_model(),
        Err(ProjectRevision3ValidationError::InvalidQuestArtifactRef { .. })
    ));

    let mut wrong_layer = base.clone();
    mutate_ref(&mut wrong_layer, &|reference| {
        reference.catalog_layer = "resolved-loadout.scripts.v1".into();
    });
    assert!(matches!(
        wrong_layer.validate_closed_model(),
        Err(ProjectRevision3ValidationError::InvalidQuestArtifactRef { .. })
    ));

    let mut wrong_semantic_length = base.clone();
    mutate_ref(&mut wrong_semantic_length, &|reference| {
        reference.source_seal.byte_len += 1;
    });
    assert!(matches!(
        wrong_semantic_length.validate_closed_model(),
        Err(ProjectRevision3ValidationError::InvalidQuestArtifactRef { .. })
    ));

    let mut oversized_artifact = base.clone();
    mutate_ref(&mut oversized_artifact, &|reference| {
        reference.artifact.byte_len = MAX_QUEST_COLLISION_ARTIFACT_BYTES + 1;
        reference.source_seal.byte_len = reference.artifact.byte_len;
    });
    assert!(matches!(
        oversized_artifact.validate_closed_model(),
        Err(ProjectRevision3ValidationError::InvalidQuestArtifactRef { .. })
    ));

    let mut wrong_basis = base.clone();
    mutate_ref(&mut wrong_basis, &|reference| {
        reference.basis_snapshot.byte_len = 0;
    });
    assert!(matches!(
        wrong_basis.validate_closed_model(),
        Err(ProjectRevision3ValidationError::InvalidQuestArtifactRef { .. })
    ));

    let mut missing = base.clone();
    missing.asset_store.assets.clear();
    assert!(matches!(
        missing.validate_closed_model(),
        Err(ProjectRevision3ValidationError::MissingQuestArtifact { .. })
    ));

    let mut wrong_size = base.clone();
    wrong_size
        .asset_store
        .assets
        .get_mut(&artifact_digest)
        .unwrap()
        .byte_len += 1;
    assert!(matches!(
        wrong_size.validate_closed_model(),
        Err(ProjectRevision3ValidationError::QuestArtifactMetadataMismatch { .. })
    ));

    let mut wrong_media = base;
    wrong_media
        .asset_store
        .assets
        .get_mut(&artifact_digest)
        .unwrap()
        .media_type = "application/json".into();
    assert!(matches!(
        wrong_media.validate_closed_model(),
        Err(ProjectRevision3ValidationError::QuestArtifactMetadataMismatch { .. })
    ));
}

#[test]
fn project_and_entity_size_caps_are_unchanged() {
    let oversized_project = " ".repeat(MAX_PROJECT_JSON_BYTES + 1);
    assert!(matches!(
        ProjectRevision3::from_json(&oversized_project),
        Err(ProjectRevision3JsonError::InputTooLarge {
            actual,
            limit: MAX_PROJECT_JSON_BYTES,
        }) if actual == MAX_PROJECT_JSON_BYTES + 1
    ));

    let mut oversized_programmatic_project = empty_revision3();
    oversized_programmatic_project.meta.name = "x".repeat(MAX_PROJECT_JSON_BYTES + 1);
    assert!(matches!(
        oversized_programmatic_project.to_canonical_json(),
        Err(ProjectRevision3JsonError::InputTooLarge {
            limit: MAX_PROJECT_JSON_BYTES,
            ..
        })
    ));

    let mut project = quest_project();
    let Revision3EntityPayload::ScriptModule(module) =
        &mut project.entities.get_mut(&entity_id(11)).unwrap().payload
    else {
        panic!("expected module")
    };
    module.source = "x".repeat(MAX_REVISION3_ENTITY_JSON_BYTES + 1);
    assert!(matches!(
        project.validate_closed_model(),
        Err(ProjectRevision3ValidationError::EntityTooLarge {
            max: MAX_REVISION3_ENTITY_JSON_BYTES,
            ..
        })
    ));
    let untrusted = serde_json::to_string(&project).unwrap();
    assert!(matches!(
        ProjectRevision3::from_json(&untrusted),
        Err(ProjectRevision3JsonError::InvalidJson(_))
    ));
}
