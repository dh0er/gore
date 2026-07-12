use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::model_revision2::{
    Entity, EntityKind, EntityPayload, NpcDraft, NpcDraftInput, NpcParentClassInput, OriginRef,
    ProjectRevision2, QuestCollisionCatalogInput, QuestDraft, QuestDraftInput, QuestGiverInput,
    QuestParentInput, SchemaRevisionV2, ScriptModule, TypedRef,
};
use gore_authoring::{
    AssetStoreIndex, ContentSeal, DiagnosticCode, DiagnosticSeverity, EntityId, FormatV2,
    GameGenerationAnchor, ProjectId, ProjectMeta, Sha256Digest, ValidationProfile,
    DRAFT_QUEST_GENERATOR_ID, DRAFT_QUEST_GENERATOR_VERSION, LOGICAL_NPC_CLONE_GENERATOR_ID,
    LOGICAL_NPC_CLONE_GENERATOR_VERSION,
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

fn generation(value: u8) -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: seal(value, 1_000_000),
    }
}

fn npc_parent(
    target: &GameGenerationAnchor,
    seal_byte: u8,
    selector: &str,
    runtime_class: &str,
) -> NpcParentClassInput {
    NpcParentClassInput {
        generation: target.clone(),
        source_seal: seal(seal_byte, 20_000),
        catalog_layer: "base-game.g1r.characters".into(),
        canonical_selector: selector.into(),
        runtime_class: runtime_class.into(),
    }
}

fn npc_draft(project: ProjectId, target: &GameGenerationAnchor) -> NpcDraft {
    NpcDraft {
        generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.into(),
        generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
        input: NpcDraftInput {
            target: target.clone(),
            module_namespace: "GoreMods.Npcs.AsghanClone".into(),
            unique_name: "GoreAsghanClone".into(),
            parent_character_definition: npc_parent(
                target,
                2,
                "CatalogCharacterDefinition_Asghan",
                "UCharacterDefinition_Human_Asghan",
            ),
            parent_ai_agent_config: npc_parent(
                target,
                3,
                "CatalogAiAgentConfig_Asghan",
                "UAIAgentConfig_Human_Asghan",
            ),
            parent_spawn_definition: npc_parent(
                target,
                4,
                "CatalogSpawnDefinition_Asghan",
                "USpawnAIAgentDefinition_Asghan",
            ),
        },
        script_module: TypedRef::new(project, entity_id(11), EntityKind::ScriptModule),
    }
}

fn quest_draft(project: ProjectId, target: &GameGenerationAnchor) -> QuestDraft {
    QuestDraft {
        generator_id: DRAFT_QUEST_GENERATOR_ID.into(),
        generator_version: DRAFT_QUEST_GENERATOR_VERSION,
        input: QuestDraftInput {
            target: target.clone(),
            quest_id: entity_id(20),
            module_namespace: "GoreMods.Quests.AsghanTrial".into(),
            technical_id: "GORE_ASGHAN_TRIAL".into(),
            text_helper: "GoreAsghanTrialText".into(),
            parent_quest: QuestParentInput {
                generation: target.clone(),
                source_seal: seal(5, 30_000),
                catalog_layer: "base-game.g1r.quests".into(),
                canonical_selector: "CatalogQuest_AsghanParent".into(),
                runtime_class: "UQuest_SwampCamp_SCCHAPTER2".into(),
            },
            giver: QuestGiverInput {
                generation: target.clone(),
                source_seal: seal(6, 40_000),
                catalog_layer: "base-game.g1r.characters".into(),
                canonical_selector: "CatalogCharacter_Asghan".into(),
                runtime_unique_name: "OM_GRD_Asghan_263".into(),
            },
            title: "Asghan's Trial".into(),
            description: "Prove that the gate is secure.".into(),
            objective_title: "Report to Asghan".into(),
            collision_catalog: QuestCollisionCatalogInput {
                generation: target.clone(),
                source_seal: seal(7, 50_000),
                catalog_layer: "resolved-loadout.scripts.v1".into(),
                modules: BTreeSet::from(["existing.module".into()]),
                relative_paths: BTreeSet::from(["existing/module.as".into()]),
                symbols: BTreeSet::from(["uexistingsymbol".into()]),
            },
        },
        script_module: TypedRef::new(project, entity_id(21), EntityKind::ScriptModule),
    }
}

fn story_project() -> ProjectRevision2 {
    let project_id = project_id(1);
    let target = generation(1);
    let npc = npc_draft(project_id, &target);
    let quest = quest_draft(project_id, &target);
    let npc_owner = TypedRef::new(project_id, entity_id(10), EntityKind::NpcDraft);
    let quest_owner = TypedRef::new(project_id, entity_id(20), EntityKind::QuestDraft);
    let npc_script = npc.regenerate_script_module(npc_owner.clone()).unwrap();
    let quest_script = quest.regenerate_script_module(quest_owner.clone()).unwrap();

    let entities = BTreeMap::from([
        (
            entity_id(10),
            Entity {
                id: entity_id(10),
                display_name: "Asghan clone".into(),
                origin: OriginRef::New {
                    authored_runtime_id: "GoreAsghanClone".into(),
                },
                revision: 0,
                payload: EntityPayload::NpcDraft(npc),
            },
        ),
        (
            entity_id(11),
            Entity {
                id: entity_id(11),
                display_name: "Asghan clone script".into(),
                origin: OriginRef::Generated {
                    generator_id: npc_script.generator_id.clone(),
                    generator_version: npc_script.generator_version,
                    owner: npc_owner,
                },
                revision: 0,
                payload: EntityPayload::ScriptModule(npc_script),
            },
        ),
        (
            entity_id(20),
            Entity {
                id: entity_id(20),
                display_name: "Asghan's Trial".into(),
                origin: OriginRef::New {
                    authored_runtime_id: "GORE_ASGHAN_TRIAL".into(),
                },
                revision: 0,
                payload: EntityPayload::QuestDraft(quest),
            },
        ),
        (
            entity_id(21),
            Entity {
                id: entity_id(21),
                display_name: "Asghan's Trial script".into(),
                origin: OriginRef::Generated {
                    generator_id: quest_script.generator_id.clone(),
                    generator_version: quest_script.generator_version,
                    owner: quest_owner,
                },
                revision: 0,
                payload: EntityPayload::ScriptModule(quest_script),
            },
        ),
    ]);

    ProjectRevision2 {
        format: FormatV2,
        schema_revision: SchemaRevisionV2,
        project_id,
        revision: 0,
        meta: ProjectMeta {
            name: "Story graph".into(),
            version: "0.1".into(),
            author: "test".into(),
        },
        target,
        authoring_locales: BTreeSet::new(),
        entities,
        asset_store: AssetStoreIndex::default(),
    }
}

fn script_mut(project: &mut ProjectRevision2, id: u8) -> &mut ScriptModule {
    let entity = project.entities.get_mut(&entity_id(id)).unwrap();
    let EntityPayload::ScriptModule(module) = &mut entity.payload else {
        panic!("expected script module");
    };
    module
}

fn invalid_draft_origins(
    target: &GameGenerationAnchor,
    owner: EntityId,
    owner_kind: EntityKind,
) -> Vec<(&'static str, OriginRef)> {
    vec![
        (
            "empty new id",
            OriginRef::New {
                authored_runtime_id: String::new(),
            },
        ),
        (
            "mismatched new id",
            OriginRef::New {
                authored_runtime_id: "wrong".into(),
            },
        ),
        (
            "vanilla",
            OriginRef::Vanilla {
                generation: target.clone(),
                catalog_layer: "base-game.g1r".into(),
                canonical_selector: "CatalogEntry".into(),
                source_seal: seal(40, 100),
            },
        ),
        (
            "imported",
            OriginRef::Imported {
                importer: "test-importer".into(),
                source_seal: seal(41, 100),
                external_identity: Some("external".into()),
            },
        ),
        (
            "generated",
            OriginRef::Generated {
                generator_id: "forged".into(),
                generator_version: 1,
                owner: TypedRef::new(project_id(1), owner, owner_kind),
            },
        ),
    ]
}

#[test]
fn npc_and_quest_graphs_regenerate_exactly_and_remain_explicitly_unqualified() {
    let project = story_project();
    let experimental =
        project.validate_story_entities_with_profile(ValidationProfile::Experimental);
    assert_eq!(experimental.len(), 2);
    assert!(experimental.iter().all(|diagnostic| {
        diagnostic.code == DiagnosticCode::RuntimeUnqualified
            && diagnostic.severity == DiagnosticSeverity::Warning
            && !diagnostic.blocks_build
    }));

    let production = project.validate_story_entities();
    assert_eq!(production.len(), 2);
    assert!(production.iter().all(|diagnostic| {
        diagnostic.code == DiagnosticCode::RuntimeUnqualified
            && diagnostic.severity == DiagnosticSeverity::Error
            && diagnostic.blocks_build
    }));
}

#[test]
fn revision2_story_json_has_one_canonical_closed_spelling() {
    let project = story_project();
    let canonical = project.to_canonical_json().unwrap();
    let reopened = ProjectRevision2::from_json(&canonical).unwrap();
    assert_eq!(reopened, project);
    assert_eq!(reopened.to_canonical_json().unwrap(), canonical);

    let unknown = canonical.replacen(
        "\"unique_name\":\"GoreAsghanClone\"",
        "\"unique_name\":\"GoreAsghanClone\",\"runtime_evidence\":true",
        1,
    );
    assert!(ProjectRevision2::from_json(&unknown).is_err());

    let duplicate = canonical.replacen(
        "\"generator_id\":\"gore-authoring.logical-npc-clone-draft\"",
        concat!(
            "\"generator_id\":\"gore-authoring.logical-npc-clone-draft\",",
            "\"generator_id\":\"shadowed\""
        ),
        1,
    );
    assert!(ProjectRevision2::from_json(&duplicate).is_err());

    let duplicate_collision = canonical.replacen(
        "\"modules\":[\"existing.module\"]",
        "\"modules\":[\"existing.module\",\"existing.module\"]",
        1,
    );
    assert!(ProjectRevision2::from_json(&duplicate_collision).is_err());

    let unknown_kind = canonical.replacen("\"kind\":\"npc_draft\"", "\"kind\":\"npc_runtime\"", 1);
    assert!(ProjectRevision2::from_json(&unknown_kind).is_err());
}

#[test]
fn collision_inventory_is_bounded_during_deserialization() {
    let canonical = story_project().to_canonical_json().unwrap();
    let oversized = "x".repeat(513);
    let malicious = canonical.replacen(
        "\"modules\":[\"existing.module\"]",
        &format!("\"modules\":[\"{oversized}\"]"),
        1,
    );
    let error = ProjectRevision2::from_json(&malicious).unwrap_err();
    assert!(error.to_string().contains("maximum is 512"));
}

#[test]
fn generated_source_sha_and_fingerprint_drift_always_block() {
    let mut source_drift = story_project();
    script_mut(&mut source_drift, 11)
        .source
        .push_str("// drift\n");
    let diagnostics =
        source_drift.validate_story_entities_with_profile(ValidationProfile::Experimental);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::GeneratedScriptDrift
            && diagnostic.property_path.as_deref() == Some("payload.data.source")
            && diagnostic.blocks_build
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::GeneratedScriptDrift
            && diagnostic.property_path.as_deref() == Some("payload.data.source_sha256")
            && diagnostic.blocks_build
    }));

    let mut seal_drift = story_project();
    let npc = seal_drift.entities.get_mut(&entity_id(10)).unwrap();
    let EntityPayload::NpcDraft(npc) = &mut npc.payload else {
        panic!("expected NPC draft");
    };
    npc.input.parent_character_definition.source_seal.sha256 = Sha256Digest::from_bytes([99; 32]);
    let diagnostics =
        seal_drift.validate_story_entities_with_profile(ValidationProfile::Experimental);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::GeneratedScriptDrift
            && diagnostic.property_path.as_deref() == Some("payload.data.input_fingerprint")
            && diagnostic.blocks_build
    }));
}

#[test]
fn generator_ref_owner_and_origin_manipulation_fail_closed() {
    let mut generator_drift = story_project();
    let npc = generator_drift.entities.get_mut(&entity_id(10)).unwrap();
    let EntityPayload::NpcDraft(npc) = &mut npc.payload else {
        panic!("expected NPC draft");
    };
    npc.generator_version += 1;
    let diagnostics =
        generator_drift.validate_story_entities_with_profile(ValidationProfile::Experimental);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::GeneratorContractDrift && diagnostic.blocks_build
    }));

    let mut foreign_ref = story_project();
    let npc = foreign_ref.entities.get_mut(&entity_id(10)).unwrap();
    let EntityPayload::NpcDraft(npc) = &mut npc.payload else {
        panic!("expected NPC draft");
    };
    npc.script_module.project_id = project_id(9);
    let diagnostics =
        foreign_ref.validate_story_entities_with_profile(ValidationProfile::Experimental);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::ReferenceProjectMismatch && diagnostic.blocks_build
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::ScriptModuleOwnershipMismatch && diagnostic.blocks_build
    }));

    let mut origin_drift = story_project();
    origin_drift
        .entities
        .get_mut(&entity_id(21))
        .unwrap()
        .origin = OriginRef::New {
        authored_runtime_id: "forged".into(),
    };
    let diagnostics =
        origin_drift.validate_story_entities_with_profile(ValidationProfile::Experimental);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::GeneratorContractDrift
            && diagnostic.property_path.as_deref() == Some("origin")
            && diagnostic.blocks_build
    }));
}

#[test]
fn invalid_seals_ids_and_statuses_cannot_be_promoted_by_experimental_policy() {
    let mut invalid_seal = story_project();
    let quest = invalid_seal.entities.get_mut(&entity_id(20)).unwrap();
    let EntityPayload::QuestDraft(quest) = &mut quest.payload else {
        panic!("expected quest draft");
    };
    quest.input.giver.source_seal.byte_len = 0;
    quest.input.quest_id = entity_id(99);
    let diagnostics =
        invalid_seal.validate_story_entities_with_profile(ValidationProfile::Experimental);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::InvalidGeneratorInput && diagnostic.blocks_build
    }));

    let canonical = story_project().to_canonical_json().unwrap();
    let qualified = canonical.replacen(
        "\"runtime\":\"runtime_unqualified\"",
        "\"runtime\":\"runtime_qualified\"",
        1,
    );
    assert!(ProjectRevision2::from_json(&qualified).is_err());

    let evidence = canonical.replacen(
        "\"runtime\":\"runtime_unqualified\"",
        "\"runtime\":\"runtime_unqualified\",\"runtime_evidence\":true",
        1,
    );
    assert!(ProjectRevision2::from_json(&evidence).is_err());
}

#[test]
fn npc_and_quest_draft_origins_are_closed_new_identity_contracts() {
    for (draft_id, kind) in [
        (entity_id(10), EntityKind::NpcDraft),
        (entity_id(20), EntityKind::QuestDraft),
    ] {
        let baseline = story_project();
        for (case, origin) in invalid_draft_origins(&baseline.target, draft_id, kind) {
            let mut project = baseline.clone();
            project.entities.get_mut(&draft_id).unwrap().origin = origin;
            let diagnostics =
                project.validate_story_entities_with_profile(ValidationProfile::Experimental);
            let invalid = diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic.code == DiagnosticCode::InvalidOrigin
                        && diagnostic.entity == Some(draft_id)
                })
                .collect::<Vec<_>>();
            assert_eq!(invalid.len(), 1, "{kind:?}: {case}");
            assert_eq!(
                invalid[0].property_path.as_deref(),
                Some("origin"),
                "{case}"
            );
            assert_eq!(invalid[0].severity, DiagnosticSeverity::Error, "{case}");
            assert!(invalid[0].blocks_build, "{case}");
        }
    }
}

#[test]
fn orphan_shared_and_wrongly_declared_script_refs_fail_closed() {
    let mut orphan = story_project();
    orphan.entities.remove(&entity_id(10));
    let diagnostics = orphan.validate_story_entities_with_profile(ValidationProfile::Experimental);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::MissingReference
            && diagnostic.entity == Some(entity_id(11))
            && diagnostic.blocks_build
    }));

    let mut shared = story_project();
    let mut second = shared.entities.get(&entity_id(10)).unwrap().clone();
    second.id = entity_id(12);
    shared.entities.insert(entity_id(12), second);
    let diagnostics = shared.validate_story_entities_with_profile(ValidationProfile::Experimental);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::GeneratedScriptDrift
            && diagnostic.entity == Some(entity_id(11))
            && diagnostic.property_path.as_deref() == Some("payload.data.owner")
            && diagnostic.blocks_build
    }));

    let mut wrong_declared_kind = story_project();
    let npc = wrong_declared_kind
        .entities
        .get_mut(&entity_id(10))
        .unwrap();
    let EntityPayload::NpcDraft(npc) = &mut npc.payload else {
        panic!("expected NPC draft");
    };
    npc.script_module.expected_kind = EntityKind::VoiceTake;
    let diagnostics =
        wrong_declared_kind.validate_story_entities_with_profile(ValidationProfile::Experimental);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::ReferenceDeclaredKindMismatch
            && diagnostic.entity == Some(entity_id(10))
            && diagnostic.blocks_build
    }));
}

#[test]
fn collision_inventories_reject_casefold_aliases_and_exact_json_duplicates() {
    let mut casefold_alias = story_project();
    let quest = casefold_alias.entities.get_mut(&entity_id(20)).unwrap();
    let EntityPayload::QuestDraft(quest) = &mut quest.payload else {
        panic!("expected quest draft");
    };
    quest
        .input
        .collision_catalog
        .modules
        .insert("Existing.Module".into());
    let diagnostics =
        casefold_alias.validate_story_entities_with_profile(ValidationProfile::Experimental);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::InvalidGeneratorInput
            && diagnostic.entity == Some(entity_id(20))
            && diagnostic.blocks_build
    }));

    let canonical = story_project().to_canonical_json().unwrap();
    let duplicate = canonical.replacen(
        "\"modules\":[\"existing.module\"]",
        "\"modules\":[\"existing.module\",\"existing.module\"]",
        1,
    );
    let error = ProjectRevision2::from_json(&duplicate).unwrap_err();
    assert!(error.to_string().contains("duplicate collision set value"));
}

#[test]
fn generated_drift_diagnostics_use_the_canonical_entity_map_key() {
    let mut project = story_project();
    let script = project.entities.get_mut(&entity_id(11)).unwrap();
    script.id = entity_id(77);
    let EntityPayload::ScriptModule(module) = &mut script.payload else {
        panic!("expected script module");
    };
    module.source.push_str("// drift\n");

    let diagnostics = project.validate_story_entities_with_profile(ValidationProfile::Experimental);
    let drift = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == DiagnosticCode::GeneratedScriptDrift)
        .collect::<Vec<_>>();
    assert!(!drift.is_empty());
    assert!(drift
        .iter()
        .all(|diagnostic| diagnostic.entity == Some(entity_id(11))));
}

#[test]
fn story_diagnostics_are_deterministic() {
    let mut project = story_project();
    script_mut(&mut project, 11).source = "tampered".into();
    script_mut(&mut project, 21).generator_id = "wrong".into();
    let first = project.validate_story_entities();
    let second = project.validate_story_entities();
    assert_eq!(first, second);
}
