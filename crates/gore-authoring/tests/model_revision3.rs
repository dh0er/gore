use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::model_revision2::{
    Entity as Revision2Entity, EntityPayload as Revision2EntityPayload,
    OriginRef as Revision2OriginRef, ProjectRevision2, QuestCollisionCatalogInput,
    QuestDraft as Revision2QuestDraft, QuestDraftInput as Revision2QuestDraftInput,
    SchemaRevisionV2,
};
use gore_authoring::{
    migrate_revision2_to_revision3, AssetMeta, AssetStoreIndex, ContentSeal, EntityId, FormatV2,
    GameGenerationAnchor, ProjectDocument, ProjectId, ProjectMeta, ProjectRevision3,
    ProjectRevision3JsonError, ProjectRevision3ValidationError, QuestCollisionArtifactRef,
    Revision2QuestGiverInput, Revision2QuestParentInput, Revision2ToRevision3Error,
    Revision3Entity, Revision3EntityKind, Revision3EntityPayload, Revision3OriginRef,
    Revision3QuestDraft, Revision3QuestDraftInput, Revision3ScriptModule, Revision3TypedRef,
    SchemaRevisionV3, ScriptModuleStatus, Sha256Digest, DRAFT_QUEST_GENERATOR_ID,
    DRAFT_QUEST_GENERATOR_VERSION, LOGICAL_NPC_CLONE_GENERATOR_ID, MAX_PROJECT_JSON_BYTES,
    MAX_QUEST_COLLISION_ARTIFACT_BYTES, MAX_REVISION3_ENTITY_JSON_BYTES,
    QUEST_COLLISION_ARTIFACT_MEDIA_TYPE, QUEST_COLLISION_CATALOG_LAYER,
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

fn parent() -> Revision2QuestParentInput {
    Revision2QuestParentInput {
        generation: target(),
        source_seal: seal(2, 20_000),
        catalog_layer: "base-game.g1r.quests".into(),
        canonical_selector: "CatalogQuest_Parent".into(),
        runtime_class: "UQuest_Parent".into(),
    }
}

fn giver() -> Revision2QuestGiverInput {
    Revision2QuestGiverInput {
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
            media_type: QUEST_COLLISION_ARTIFACT_MEDIA_TYPE.into(),
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
            collision_catalog: QuestCollisionArtifactRef {
                generation: target(),
                catalog_layer: QUEST_COLLISION_CATALOG_LAYER.into(),
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

    // Dispatcher integration is intentionally deferred until the Store/FFI slice.
    assert!(ProjectDocument::from_json(&canonical).is_err());
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

    let revision2 = canonical.replacen("\"schema_revision\":3", "\"schema_revision\":2", 1);
    assert!(matches!(
        ProjectRevision3::from_json(&revision2),
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

fn empty_revision2() -> ProjectRevision2 {
    ProjectRevision2 {
        format: FormatV2,
        schema_revision: SchemaRevisionV2,
        project_id: project_id(2),
        revision: 4,
        meta: ProjectMeta {
            name: "Revision 2 source".into(),
            version: "0.1".into(),
            author: "tests".into(),
        },
        target: target(),
        authoring_locales: BTreeSet::new(),
        entities: BTreeMap::new(),
        asset_store: AssetStoreIndex::default(),
    }
}

#[test]
fn quest_free_revision2_migration_is_explicit_deterministic_and_lossless() {
    let mut source = empty_revision2();
    let localization_id = entity_id(30);
    source.entities.insert(
        localization_id,
        Revision2Entity {
            id: localization_id,
            display_name: "Migrated localization".into(),
            origin: Revision2OriginRef::New {
                authored_runtime_id: "GORE_MIGRATED_LOC".into(),
            },
            revision: 2,
            payload: Revision2EntityPayload::LocalizationEntry(
                gore_authoring::Revision2LocalizationEntry {
                    loc_id: "GORE_MIGRATED_LOC".into(),
                    texts: BTreeMap::new(),
                },
            ),
        },
    );
    source.asset_store.assets.insert(
        seal(0x31, 64).sha256,
        AssetMeta {
            byte_len: 64,
            media_type: "application/octet-stream".into(),
        },
    );
    let source_before = source.to_canonical_json().unwrap();
    let first = migrate_revision2_to_revision3(&source).unwrap();
    let second = migrate_revision2_to_revision3(&source).unwrap();

    assert_eq!(first, second);
    assert_eq!(source.to_canonical_json().unwrap(), source_before);
    assert_eq!(first.project.project_id, source.project_id);
    assert_eq!(first.project.revision, source.revision);
    assert_eq!(first.project.meta, source.meta);
    assert_eq!(first.project.target, source.target);
    assert_eq!(first.project.asset_store, source.asset_store);
    assert_eq!(first.report.migrated_entities, 1);
    assert_eq!(first.report.collision_artifacts_created, 0);
    let Revision3EntityPayload::LocalizationEntry(localization) =
        &first.project.entities[&localization_id].payload
    else {
        panic!("expected migrated localization")
    };
    assert_eq!(localization.loc_id, "GORE_MIGRATED_LOC");
    let canonical = first.project.to_canonical_json().unwrap();
    assert_eq!(
        ProjectRevision3::from_json(&canonical).unwrap(),
        first.project
    );
}

fn revision2_quest(id: EntityId) -> Revision2Entity {
    Revision2Entity {
        id,
        display_name: "Legacy Quest".into(),
        origin: Revision2OriginRef::New {
            authored_runtime_id: "LEGACY_QUEST".into(),
        },
        revision: 0,
        payload: Revision2EntityPayload::QuestDraft(Revision2QuestDraft {
            generator_id: "gore-authoring.draft-quest-skeleton".into(),
            generator_version: 1,
            input: Revision2QuestDraftInput {
                target: target(),
                quest_id: id,
                module_namespace: "Legacy.Quest".into(),
                technical_id: "LEGACY_QUEST".into(),
                text_helper: "LegacyText".into(),
                parent_quest: parent(),
                giver: giver(),
                title: "Legacy".into(),
                description: "Legacy description".into(),
                objective_title: "Legacy objective".into(),
                collision_catalog: QuestCollisionCatalogInput {
                    generation: target(),
                    source_seal: seal(8, 100),
                    catalog_layer: "base-game-plus-exact-project.story-collisions.v1".into(),
                    modules: BTreeSet::new(),
                    relative_paths: BTreeSet::new(),
                    symbols: BTreeSet::new(),
                },
            },
            script_module: Revision3TypedRef::new(
                project_id(2),
                entity_id(99),
                Revision3EntityKind::ScriptModule,
            ),
        }),
    }
}

#[test]
fn any_revision2_quest_requires_explicit_repinning_and_returns_no_candidate() {
    let mut source = empty_revision2();
    source
        .entities
        .insert(entity_id(10), revision2_quest(entity_id(10)));
    source
        .entities
        .insert(entity_id(20), revision2_quest(entity_id(20)));

    assert!(matches!(
        migrate_revision2_to_revision3(&source),
        Err(Revision2ToRevision3Error::QuestRepinRequired {
            quest_count: 2,
            first_quest,
        }) if first_quest == entity_id(10)
    ));
}

fn revision2_script_module(
    module_id: EntityId,
    owner_id: EntityId,
    owner_kind: Revision3EntityKind,
    generator_id: &str,
    generator_version: u32,
) -> Revision2Entity {
    let owner = Revision3TypedRef::new(project_id(2), owner_id, owner_kind);
    let source = "// residual revision-2 module\n".to_owned();
    Revision2Entity {
        id: module_id,
        display_name: "Residual generated module".into(),
        origin: Revision2OriginRef::Generated {
            generator_id: generator_id.into(),
            generator_version,
            owner: owner.clone(),
        },
        revision: 0,
        payload: Revision2EntityPayload::ScriptModule(Revision3ScriptModule {
            generator_id: generator_id.into(),
            generator_version,
            owner,
            module_namespace: "Legacy.Residual".into(),
            module_relative_path: "Legacy/Residual.as".into(),
            source_sha256: Sha256Digest::from_bytes(Sha256::digest(source.as_bytes()).into()),
            source,
            input_fingerprint: Sha256Digest::from_bytes([0x71; 32]),
            status: ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED,
        }),
    }
}

#[test]
fn orphan_quest_owned_revision2_module_requires_explicit_repinning() {
    let mut source = empty_revision2();
    let module_id = entity_id(40);
    let missing_owner = entity_id(41);
    source.entities.insert(
        module_id,
        revision2_script_module(
            module_id,
            missing_owner,
            Revision3EntityKind::QuestDraft,
            LOGICAL_NPC_CLONE_GENERATOR_ID,
            1,
        ),
    );

    assert!(matches!(
        migrate_revision2_to_revision3(&source),
        Err(Revision2ToRevision3Error::QuestModuleRepinRequired { module, owner })
            if module == module_id && owner == missing_owner
    ));
}

#[test]
fn quest_generator_marker_requires_repin_even_with_non_quest_owner_or_version_drift() {
    for (index, generator_version) in [
        DRAFT_QUEST_GENERATOR_VERSION,
        DRAFT_QUEST_GENERATOR_VERSION + 99,
    ]
    .into_iter()
    .enumerate()
    {
        let mut source = empty_revision2();
        let module_id = entity_id(50 + u8::try_from(index).unwrap());
        let owner_id = entity_id(60 + u8::try_from(index).unwrap());
        source.entities.insert(
            module_id,
            revision2_script_module(
                module_id,
                owner_id,
                Revision3EntityKind::NpcDraft,
                DRAFT_QUEST_GENERATOR_ID,
                generator_version,
            ),
        );

        assert!(matches!(
            migrate_revision2_to_revision3(&source),
            Err(Revision2ToRevision3Error::QuestModuleRepinRequired { module, owner })
                if module == module_id && owner == owner_id
        ));
    }
}

#[test]
fn mismatched_quest_origin_marker_cannot_hide_behind_non_quest_module_fields() {
    let mut source = empty_revision2();
    let module_id = entity_id(70);
    let owner_id = entity_id(71);
    let mut entity = revision2_script_module(
        module_id,
        owner_id,
        Revision3EntityKind::NpcDraft,
        LOGICAL_NPC_CLONE_GENERATOR_ID,
        1,
    );
    let Revision2OriginRef::Generated {
        generator_id,
        generator_version,
        ..
    } = &mut entity.origin
    else {
        panic!("expected generated origin")
    };
    *generator_id = DRAFT_QUEST_GENERATOR_ID.into();
    *generator_version = DRAFT_QUEST_GENERATOR_VERSION;
    source.entities.insert(module_id, entity);

    assert!(matches!(
        migrate_revision2_to_revision3(&source),
        Err(Revision2ToRevision3Error::QuestModuleRepinRequired { module, owner })
            if module == module_id && owner == owner_id
    ));
}
