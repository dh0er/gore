use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::{
    AssetMeta, AssetStoreIndex, ContentSeal, EntityId, FormatV2, GameGenerationAnchor, ProjectId,
    ProjectMeta, ProjectRevision3, QuestCollisionArtifactRef, QuestTransitionPlanV1,
    Revision2DialogLine, Revision2LocalizationEntry, Revision2NpcParentClassInput, Revision3Entity,
    Revision3EntityKind, Revision3EntityPayload, Revision3NpcDraft, Revision3NpcDraftInput,
    Revision3NpcGreetingBindingV1, Revision3OriginRef, Revision3QuestDraft,
    Revision3QuestDraftInput, Revision3QuestGiverInput, Revision3QuestParentInput,
    Revision3ScriptModule, Revision3TypedRef, SchemaRevisionV3, ScriptModuleStatus, Sha256Digest,
    LOGICAL_NPC_CLONE_GENERATOR_ID, LOGICAL_NPC_CLONE_GENERATOR_VERSION,
    MAX_ANGELSCRIPT_IDENTIFIER_BYTES, MAX_REVISION3_ENTITY_JSON_BYTES,
    MAX_REVISION3_NPC_DRAFT_DISPLAY_NAME_BYTES_V1, QUEST_COLLISION_ARTIFACT_MEDIA_TYPE,
    QUEST_COLLISION_CATALOG_LAYER, REVISION3_QUEST_GENERATOR_ID,
    REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION,
};
use sha2::{Digest as _, Sha256};

use crate::revision3_npc::{
    build_revision3_npc_source_inspection_plan_v1, NpcInspectionBuildStatusV1,
    NpcInspectionCompilerStatusV1, NpcInspectionDiagnosticCodeV1, NpcInspectionPublicationStatusV1,
    NpcInspectionRuntimeQualificationV1, NpcInspectionScopeV1, NpcInspectionSourceStatusV1,
    NpcInspectionSpawnStatusV1, Revision3NpcInspectionDiagnosticV1, Revision3NpcInspectionEntityV1,
    Revision3NpcInspectionErrorV1, Revision3NpcInspectionModuleV1,
    Revision3NpcInspectionProvenanceV1, Revision3NpcSourceInspectionPlanV1,
    MAX_REVISION3_NPC_INSPECTION_PLAN_JSON_BYTES, MAX_REVISION3_NPC_INSPECTION_PROJECT_JSON_BYTES,
};

const NPC_ID: u8 = 10;
const NPC_MODULE_ID: u8 = 11;
const QUEST_ID: u8 = 20;
const QUEST_MODULE_ID: u8 = 21;

trait AmbiguousIfDeserialize<Marker> {
    fn marker() {}
}

impl<T: ?Sized> AmbiguousIfDeserialize<()> for T {}
impl<T> AmbiguousIfDeserialize<u8> for T where T: for<'de> serde::Deserialize<'de> {}

fn project_id(value: u8) -> ProjectId {
    ProjectId::from_bytes([value; 16])
}

fn entity_id(value: u8) -> EntityId {
    EntityId::from_bytes([value; 16])
}

fn digest(value: u8) -> Sha256Digest {
    Sha256Digest::from_bytes([value; 32])
}

fn seal(value: u8, byte_len: u64) -> ContentSeal {
    ContentSeal {
        byte_len,
        sha256: digest(value),
    }
}

fn sha256(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::from_bytes(Sha256::digest(bytes).into())
}

fn target() -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: seal(1, 171_698_176),
    }
}

fn npc_parent(value: u8, runtime_class: &str) -> Revision2NpcParentClassInput {
    Revision2NpcParentClassInput {
        generation: target(),
        source_seal: seal(value, 4_096),
        catalog_layer: "base-game.g1r.npc-parents.v1".to_owned(),
        canonical_selector: runtime_class.to_owned(),
        runtime_class: runtime_class.to_owned(),
    }
}

fn npc(project: ProjectId) -> Revision3NpcDraft {
    Revision3NpcDraft {
        generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
        generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
        input: Revision3NpcDraftInput {
            target: target(),
            module_namespace: "GoreMods.Npcs.GateGuard".to_owned(),
            unique_name: "GORE_GATE_GUARD".to_owned(),
            parent_character_definition: npc_parent(
                2,
                "UCharacterDefinition_Human_OM_GRD_Asghan_263",
            ),
            parent_ai_agent_config: npc_parent(3, "UAIAgentConfig_Human_OM_GRD_Asghan_263"),
            parent_spawn_definition: npc_parent(4, "USpawnAIAgentDefinition_OM_GRD_Asghan_263"),
        },
        script_module: Revision3TypedRef::new(
            project,
            entity_id(NPC_MODULE_ID),
            Revision3EntityKind::ScriptModule,
        ),
        greetings: Vec::new(),
    }
}

fn insert_unrelated_v4_quest(project: &mut ProjectRevision3) {
    let quest_id = entity_id(QUEST_ID);
    let module_id = entity_id(QUEST_MODULE_ID);
    let artifact = seal(0x70, 4_096);
    project.asset_store.assets.insert(
        artifact.sha256,
        AssetMeta {
            byte_len: artifact.byte_len,
            media_type: QUEST_COLLISION_ARTIFACT_MEDIA_TYPE.to_owned(),
        },
    );

    let owner = Revision3TypedRef::new(
        project.project_id,
        quest_id,
        Revision3EntityKind::QuestDraft,
    );
    let draft = Revision3QuestDraft {
        generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
        generator_version: REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION,
        input: Revision3QuestDraftInput {
            target: target(),
            quest_id,
            module_namespace: "GoreMods.Quests.UnrelatedV4".to_owned(),
            technical_id: "GORE_UNRELATED_V4_QUEST".to_owned(),
            text_helper: "GoreQuestText".to_owned(),
            parent_quest: Revision3QuestParentInput {
                generation: target(),
                source_seal: seal(0x71, 2_048),
                catalog_layer: "base-game.g1r.scripts".to_owned(),
                canonical_selector: "CatalogQuestParent".to_owned(),
                runtime_class: "UQuest_SwampCamp_SCCHAPTER2".to_owned(),
            },
            giver: Revision3QuestGiverInput {
                generation: target(),
                source_seal: seal(0x72, 2_048),
                catalog_layer: "base-game.g1r.scripts".to_owned(),
                canonical_selector: "CatalogAsghan".to_owned(),
                runtime_unique_name: "OM_GRD_Asghan_263".to_owned(),
            },
            title: "Unrelated semantic Quest".to_owned(),
            description: "This V4 Quest must remain outside NPC inspection.".to_owned(),
            objective_title: "Remain untouched".to_owned(),
            additional_objective_titles: Vec::new(),
            transition_plan: Some(Box::new(
                QuestTransitionPlanV1::legacy_seed(1).expect("one-objective semantic plan"),
            )),
            collision_catalog: QuestCollisionArtifactRef {
                generation: target(),
                catalog_layer: QUEST_COLLISION_CATALOG_LAYER.to_owned(),
                artifact,
                source_seal: seal(0x73, 4_096),
                basis_snapshot: seal(0x74, 8_192),
            },
        },
        script_module: Revision3TypedRef::new(
            project.project_id,
            module_id,
            Revision3EntityKind::ScriptModule,
        ),
        transcript: Vec::new(),
    };
    let source = "// unrelated V4 Quest source; intentionally opaque to NPC inspection\n";
    let module = Revision3ScriptModule {
        generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
        generator_version: REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION,
        owner: owner.clone(),
        module_namespace: draft.input.module_namespace.clone(),
        module_relative_path: "GoreMods/Quests/UnrelatedV4.as".to_owned(),
        source: source.to_owned(),
        source_sha256: sha256(source.as_bytes()),
        input_fingerprint: digest(0x75),
        status: ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED,
    };

    project.entities.insert(
        quest_id,
        Revision3Entity {
            id: quest_id,
            display_name: "Unrelated semantic Quest".to_owned(),
            origin: Revision3OriginRef::New {
                authored_runtime_id: draft.input.technical_id.clone(),
            },
            revision: 4,
            payload: Revision3EntityPayload::QuestDraft(draft),
        },
    );
    project.entities.insert(
        module_id,
        Revision3Entity {
            id: module_id,
            display_name: "Unrelated semantic Quest source".to_owned(),
            origin: Revision3OriginRef::Generated {
                generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
                generator_version: REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION,
                owner,
            },
            revision: 5,
            payload: Revision3EntityPayload::ScriptModule(module),
        },
    );
}

fn project() -> ProjectRevision3 {
    let project_id = project_id(8);
    let npc_id = entity_id(NPC_ID);
    let npc_owner = Revision3TypedRef::new(project_id, npc_id, Revision3EntityKind::NpcDraft);
    let draft = npc(project_id);
    let module = draft
        .regenerate_script_module(npc_owner.clone())
        .expect("valid exact NPC fixture");
    let mut project = ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id,
        revision: 7,
        meta: ProjectMeta {
            name: "NPC inspection fixture".to_owned(),
            version: "0.1.0".to_owned(),
            author: "test".to_owned(),
        },
        target: target(),
        authoring_locales: BTreeSet::new(),
        entities: BTreeMap::from([
            (
                npc_id,
                Revision3Entity {
                    id: npc_id,
                    display_name: "Gate Guard".to_owned(),
                    origin: Revision3OriginRef::New {
                        authored_runtime_id: draft.input.unique_name.clone(),
                    },
                    revision: 2,
                    payload: Revision3EntityPayload::NpcDraft(draft),
                },
            ),
            (
                entity_id(NPC_MODULE_ID),
                Revision3Entity {
                    id: entity_id(NPC_MODULE_ID),
                    display_name: "Gate Guard source".to_owned(),
                    origin: Revision3OriginRef::Generated {
                        generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
                        generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
                        owner: npc_owner,
                    },
                    revision: 3,
                    payload: Revision3EntityPayload::ScriptModule(module),
                },
            ),
        ]),
        asset_store: AssetStoreIndex::default(),
    };
    insert_unrelated_v4_quest(&mut project);
    project
}

fn canonical_plan_value() -> serde_json::Value {
    let canonical_project = project().to_canonical_json().unwrap();
    let plan = build_revision3_npc_source_inspection_plan_v1(&canonical_project, entity_id(NPC_ID))
        .unwrap();
    serde_json::from_str(&plan.to_canonical_json().unwrap()).unwrap()
}

fn assert_bounded_wire_rejection(
    base: &serde_json::Value,
    label: &str,
    mutate: impl FnOnce(&mut serde_json::Value),
) -> String {
    let mut value = base.clone();
    mutate(&mut value);
    let json = serde_json::to_string(&value).unwrap();
    assert!(
        json.len() <= MAX_REVISION3_NPC_INSPECTION_PLAN_JSON_BYTES,
        "{label} must reach the bounded nested parser rather than the top-level limit"
    );
    match Revision3NpcSourceInspectionPlanV1::from_json(&json) {
        Err(Revision3NpcInspectionErrorV1::InvalidPlanJson(error)) => error.to_string(),
        other => panic!("{label} did not fail during bounded wire parsing: {other:?}"),
    }
}

#[test]
fn exact_plan_is_deterministic_read_only_and_excludes_unrelated_v4_quest_content() {
    let project = project();
    let canonical_project = project
        .to_canonical_json()
        .expect("fixture is a closed revision-3 project");
    let before = canonical_project.clone();
    let first =
        build_revision3_npc_source_inspection_plan_v1(&canonical_project, entity_id(NPC_ID))
            .expect("exact NPC inspection");
    let second =
        build_revision3_npc_source_inspection_plan_v1(&canonical_project, entity_id(NPC_ID))
            .expect("deterministic exact NPC inspection");

    assert_eq!(first, second);
    assert_eq!(canonical_project, before);
    assert_eq!(first.format(), "revision3_npc_source_inspection_plan");
    assert_eq!(first.schema_revision(), 1);
    assert_eq!(
        first.scope(),
        NpcInspectionScopeV1::SourceReadinessInspectionOnly
    );
    assert_eq!(
        first.source_status(),
        NpcInspectionSourceStatusV1::PersistedAndRegeneratedExact
    );
    assert_eq!(
        first.compiler_status(),
        NpcInspectionCompilerStatusV1::NotRun
    );
    assert_eq!(first.build_status(), NpcInspectionBuildStatusV1::Blocked);
    assert_eq!(
        first.runtime_qualification(),
        NpcInspectionRuntimeQualificationV1::RuntimeUnqualified
    );
    assert_eq!(
        first.spawn_status(),
        NpcInspectionSpawnStatusV1::NotSupported
    );
    assert_eq!(
        first.publication_status(),
        NpcInspectionPublicationStatusV1::NotSupported
    );
    assert_eq!(first.provenance().project_revision(), 7);
    assert_eq!(first.npc().entity_revision(), 2);
    assert_eq!(first.module().entity_revision(), 3);

    let regenerated = Revision3NpcDraft {
        generator_id: first.npc().generator_id().to_owned(),
        generator_version: first.npc().generator_version(),
        input: first.npc().input().clone(),
        script_module: first.npc().script_module().clone(),
        greetings: Vec::new(),
    }
    .regenerate_script_module(first.npc().reference().clone())
    .expect("persisted parent triple remains independently regenerable");
    assert_eq!(first.module().generated(), &regenerated);
    assert_eq!(
        first.module().persisted_source().sha256,
        regenerated.source_sha256
    );

    let codes: Vec<_> = first
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.code())
        .collect();
    assert_eq!(
        codes,
        vec![
            NpcInspectionDiagnosticCodeV1::NpcCompilerNotRun,
            NpcInspectionDiagnosticCodeV1::NpcProductionLoweringUnavailable,
            NpcInspectionDiagnosticCodeV1::NpcRuntimeResidenceUnqualified,
            NpcInspectionDiagnosticCodeV1::NpcSpawnUnavailable,
        ]
    );
    assert!(first
        .diagnostics()
        .iter()
        .all(|diagnostic| diagnostic.blocks_build()));

    let canonical_plan = first.to_canonical_json().expect("canonical plan");
    assert_eq!(
        Revision3NpcSourceInspectionPlanV1::from_json(&canonical_plan)
            .expect("reopen canonical plan"),
        first
    );
    first
        .verify_against_project(&canonical_project)
        .expect("plan remains bound to the exact project");
    assert_eq!(
        first.content_seal().unwrap(),
        second.content_seal().unwrap()
    );
    assert!(!canonical_plan.contains("GORE_UNRELATED_V4_QUEST"));
    assert!(!canonical_plan.contains("unrelated V4 Quest source"));
    assert!(ProjectRevision3::from_json(&canonical_project)
        .unwrap()
        .entities
        .contains_key(&entity_id(QUEST_ID)));
}

#[test]
fn authoring_only_greetings_leave_npc_source_inspection_and_verification_exact() {
    let mut project = project();
    let localization_id = entity_id(30);
    let line_id = entity_id(31);
    project.entities.insert(
        localization_id,
        Revision3Entity {
            id: localization_id,
            display_name: "Greeting localization".to_owned(),
            origin: Revision3OriginRef::New {
                authored_runtime_id: "GORE_INSPECTION_GREETING_LOC".to_owned(),
            },
            revision: 1,
            payload: Revision3EntityPayload::LocalizationEntry(Revision2LocalizationEntry {
                loc_id: "GORE_INSPECTION_GREETING".to_owned(),
                texts: BTreeMap::new(),
            }),
        },
    );
    project.entities.insert(
        line_id,
        Revision3Entity {
            id: line_id,
            display_name: "Greeting line".to_owned(),
            origin: Revision3OriginRef::New {
                authored_runtime_id: "GORE_INSPECTION_GREETING_LINE".to_owned(),
            },
            revision: 2,
            payload: Revision3EntityPayload::DialogLine(Revision2DialogLine {
                localization: Revision3TypedRef::new(
                    project.project_id,
                    localization_id,
                    Revision3EntityKind::LocalizationEntry,
                ),
                speaker_hint: Some("Asghan".to_owned()),
                voice_slots: BTreeMap::new(),
            }),
        },
    );
    let Revision3EntityPayload::NpcDraft(npc) = &mut project
        .entities
        .get_mut(&entity_id(NPC_ID))
        .unwrap()
        .payload
    else {
        unreachable!()
    };
    npc.greetings = vec![Revision3NpcGreetingBindingV1 {
        line: Revision3TypedRef::new(project.project_id, line_id, Revision3EntityKind::DialogLine),
    }];
    let module_before = project.entities[&entity_id(NPC_MODULE_ID)].clone();
    let canonical = project.to_canonical_json().unwrap();

    let plan = build_revision3_npc_source_inspection_plan_v1(&canonical, entity_id(NPC_ID))
        .expect("greetings are outside deterministic source generation");
    plan.verify_against_project(&canonical)
        .expect("source plan remains exact against greeting-bearing project");
    assert_eq!(project.entities[&entity_id(NPC_MODULE_ID)], module_before);
    assert_eq!(
        plan.source_status(),
        NpcInspectionSourceStatusV1::PersistedAndRegeneratedExact
    );
}

#[test]
fn selection_is_exact_entity_and_kind_bound() {
    let canonical = project().to_canonical_json().unwrap();
    assert!(matches!(
        build_revision3_npc_source_inspection_plan_v1(&canonical, entity_id(99)),
        Err(Revision3NpcInspectionErrorV1::MissingNpc(id)) if id == entity_id(99)
    ));
    assert!(matches!(
        build_revision3_npc_source_inspection_plan_v1(&canonical, entity_id(NPC_MODULE_ID)),
        Err(Revision3NpcInspectionErrorV1::NotAnNpc(id)) if id == entity_id(NPC_MODULE_ID)
    ));
    assert!(matches!(
        build_revision3_npc_source_inspection_plan_v1(&canonical, entity_id(QUEST_ID)),
        Err(Revision3NpcInspectionErrorV1::NotAnNpc(id)) if id == entity_id(QUEST_ID)
    ));
}

#[test]
fn canonical_spelling_and_bounded_envelopes_fail_closed() {
    let canonical_project = project().to_canonical_json().unwrap();
    let plan = build_revision3_npc_source_inspection_plan_v1(&canonical_project, entity_id(NPC_ID))
        .unwrap();
    let canonical_plan = plan.to_canonical_json().unwrap();

    assert!(build_revision3_npc_source_inspection_plan_v1(
        &format!("{canonical_project}\n"),
        entity_id(NPC_ID),
    )
    .is_err());
    assert!(matches!(
        Revision3NpcSourceInspectionPlanV1::from_json(&format!("{canonical_plan}\n")),
        Err(Revision3NpcInspectionErrorV1::NonCanonicalPlanJson)
    ));
    assert!(matches!(
        build_revision3_npc_source_inspection_plan_v1(
            &" ".repeat(MAX_REVISION3_NPC_INSPECTION_PROJECT_JSON_BYTES + 1),
            entity_id(NPC_ID),
        ),
        Err(Revision3NpcInspectionErrorV1::ProjectJsonTooLarge { .. })
    ));
    assert!(matches!(
        Revision3NpcSourceInspectionPlanV1::from_json(
            &" ".repeat(MAX_REVISION3_NPC_INSPECTION_PLAN_JSON_BYTES + 1),
        ),
        Err(Revision3NpcInspectionErrorV1::PlanJsonTooLarge { .. })
    ));
}

#[test]
fn every_nested_string_family_is_rejected_by_the_bounded_wire_parser() {
    let base = canonical_plan_value();
    let cases = [
        ("format", "/format", 65usize),
        ("scope token", "/scope", 65),
        ("typed-ref kind", "/npc/reference/expected_kind", 65),
        ("NPC display", "/npc/display_name", 257),
        ("NPC origin type", "/npc/origin/type", 65),
        (
            "NPC origin runtime id",
            "/npc/origin/authored_runtime_id",
            65,
        ),
        ("NPC generator", "/npc/generator_id", 257),
        ("NPC module namespace", "/npc/input/module_namespace", 256),
        ("NPC unique name", "/npc/input/unique_name", 65),
        (
            "parent catalog layer",
            "/npc/input/parent_character_definition/catalog_layer",
            129,
        ),
        (
            "parent runtime class",
            "/npc/input/parent_character_definition/runtime_class",
            97,
        ),
        ("module display", "/module/display_name", 257),
        (
            "module origin generator",
            "/module/origin/generator_id",
            257,
        ),
        (
            "generated module namespace",
            "/module/generated/module_namespace",
            256,
        ),
        (
            "generated module path",
            "/module/generated/module_relative_path",
            259,
        ),
        (
            "generated authoring status",
            "/module/generated/status/authoring",
            65,
        ),
        ("diagnostic code", "/diagnostics/0/code", 65),
        ("diagnostic severity", "/diagnostics/0/severity", 65),
        (
            "diagnostic property path",
            "/diagnostics/0/property_path",
            2 * 1_024 + 1,
        ),
        (
            "diagnostic message",
            "/diagnostics/0/message",
            16 * 1_024 + 1,
        ),
    ];
    for (label, pointer, length) in cases {
        let error = assert_bounded_wire_rejection(&base, label, |value| {
            *value.pointer_mut(pointer).unwrap() = serde_json::Value::String("X".repeat(length));
        });
        assert!(error.contains("maximum is"), "{label}: {error}");
    }

    let escaped = assert_bounded_wire_rejection(&base, "escaped parent selector", |value| {
        *value
            .pointer_mut("/npc/input/parent_character_definition/canonical_selector")
            .unwrap() =
            serde_json::Value::String("\0".repeat(MAX_ANGELSCRIPT_IDENTIFIER_BYTES + 1));
    });
    assert!(escaped.contains("maximum is"), "{escaped}");

    let source = assert_bounded_wire_rejection(&base, "generated source", |value| {
        *value.pointer_mut("/module/generated/source").unwrap() =
            serde_json::Value::String("S".repeat(MAX_REVISION3_ENTITY_JSON_BYTES + 1));
    });
    assert!(source.contains("maximum is"), "{source}");

    let project_id = assert_bounded_wire_rejection(&base, "project id", |value| {
        *value.pointer_mut("/provenance/project_id").unwrap() =
            serde_json::Value::String("1".repeat(33));
    });
    assert!(project_id.contains("maximum is 32"), "{project_id}");

    let digest = assert_bounded_wire_rejection(&base, "source digest", |value| {
        *value
            .pointer_mut("/module/generated/source_sha256")
            .unwrap() = serde_json::Value::String("1".repeat(65));
    });
    assert!(digest.contains("maximum is 64"), "{digest}");
}

#[test]
fn diagnostics_are_a_fixed_bounded_sequence_during_wire_parsing() {
    let base = canonical_plan_value();

    assert_bounded_wire_rejection(&base, "three diagnostics", |value| {
        value
            .pointer_mut("/diagnostics")
            .unwrap()
            .as_array_mut()
            .unwrap()
            .pop();
    });
    assert_bounded_wire_rejection(&base, "five diagnostics", |value| {
        let diagnostics = value
            .pointer_mut("/diagnostics")
            .unwrap()
            .as_array_mut()
            .unwrap();
        diagnostics.push(diagnostics[0].clone());
    });
    assert_bounded_wire_rejection(&base, "reordered diagnostics", |value| {
        value
            .pointer_mut("/diagnostics")
            .unwrap()
            .as_array_mut()
            .unwrap()
            .swap(0, 1);
    });
    assert_bounded_wire_rejection(&base, "nonblocking diagnostic", |value| {
        *value.pointer_mut("/diagnostics/0/blocks_build").unwrap() = serde_json::Value::Bool(false);
    });
    assert_bounded_wire_rejection(&base, "unknown diagnostic field", |value| {
        value
            .pointer_mut("/diagnostics/0")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("authority".to_owned(), serde_json::Value::Bool(true));
    });
}

#[test]
fn public_output_evidence_has_no_direct_deserialization_path() {
    let _ = <Revision3NpcSourceInspectionPlanV1 as AmbiguousIfDeserialize<_>>::marker as fn();
    let _ = <Revision3NpcInspectionDiagnosticV1 as AmbiguousIfDeserialize<_>>::marker as fn();
    let _ = <Revision3NpcInspectionProvenanceV1 as AmbiguousIfDeserialize<_>>::marker as fn();
    let _ = <Revision3NpcInspectionEntityV1 as AmbiguousIfDeserialize<_>>::marker as fn();
    let _ = <Revision3NpcInspectionModuleV1 as AmbiguousIfDeserialize<_>>::marker as fn();
}

#[test]
fn authority_diagnostics_and_regeneration_evidence_cannot_be_widened() {
    let canonical_project = project().to_canonical_json().unwrap();
    let plan = build_revision3_npc_source_inspection_plan_v1(&canonical_project, entity_id(NPC_ID))
        .unwrap();

    let mut missing_diagnostic = plan.clone();
    missing_diagnostic.diagnostics.pop();
    assert!(matches!(
        missing_diagnostic.to_canonical_json(),
        Err(Revision3NpcInspectionErrorV1::PlanInvariant(_))
    ));

    let mut changed_fingerprint = plan.clone();
    changed_fingerprint.module.generated.input_fingerprint = digest(0xf0);
    assert!(matches!(
        changed_fingerprint.to_canonical_json(),
        Err(Revision3NpcInspectionErrorV1::PlanInvariant(_))
    ));

    let mut unknown_field: serde_json::Value =
        serde_json::from_str(&plan.to_canonical_json().unwrap()).unwrap();
    unknown_field
        .as_object_mut()
        .unwrap()
        .insert("compiled".to_owned(), serde_json::Value::Bool(true));
    assert!(matches!(
        Revision3NpcSourceInspectionPlanV1::from_json(
            &serde_json::to_string(&unknown_field).unwrap()
        ),
        Err(Revision3NpcInspectionErrorV1::InvalidPlanJson(_))
    ));
}

#[test]
fn mutable_internal_clones_reject_oversized_strings_before_plan_serialization() {
    let canonical_project = project().to_canonical_json().unwrap();
    let plan = build_revision3_npc_source_inspection_plan_v1(&canonical_project, entity_id(NPC_ID))
        .unwrap();

    let mut parent_selector = plan.clone();
    parent_selector
        .npc
        .input
        .parent_character_definition
        .canonical_selector = "A".repeat(MAX_ANGELSCRIPT_IDENTIFIER_BYTES + 1);
    assert!(matches!(
        parent_selector.to_canonical_json(),
        Err(Revision3NpcInspectionErrorV1::PlanFieldTooLarge {
            field: "npc.input.parent_character_definition.canonical_selector",
            actual,
            limit: MAX_ANGELSCRIPT_IDENTIFIER_BYTES,
        }) if actual == MAX_ANGELSCRIPT_IDENTIFIER_BYTES + 1
    ));

    let mut npc_display_name = plan.clone();
    npc_display_name.npc.display_name =
        "N".repeat(MAX_REVISION3_NPC_DRAFT_DISPLAY_NAME_BYTES_V1 + 1);
    assert!(matches!(
        npc_display_name.to_canonical_json(),
        Err(Revision3NpcInspectionErrorV1::PlanFieldTooLarge {
            field: "npc.display_name",
            actual,
            limit: MAX_REVISION3_NPC_DRAFT_DISPLAY_NAME_BYTES_V1,
        }) if actual == MAX_REVISION3_NPC_DRAFT_DISPLAY_NAME_BYTES_V1 + 1
    ));

    let mut module_display_name = plan.clone();
    module_display_name.module.display_name =
        "M".repeat(MAX_REVISION3_NPC_DRAFT_DISPLAY_NAME_BYTES_V1 + 1);
    assert!(matches!(
        module_display_name.to_canonical_json(),
        Err(Revision3NpcInspectionErrorV1::PlanFieldTooLarge {
            field: "module.display_name",
            actual,
            limit: MAX_REVISION3_NPC_DRAFT_DISPLAY_NAME_BYTES_V1,
        }) if actual == MAX_REVISION3_NPC_DRAFT_DISPLAY_NAME_BYTES_V1 + 1
    ));

    let mut source = plan;
    source.module.generated.source = "S".repeat(MAX_REVISION3_ENTITY_JSON_BYTES + 1);
    assert!(matches!(
        source.to_canonical_json(),
        Err(Revision3NpcInspectionErrorV1::PlanFieldTooLarge {
            field: "module.generated.source",
            actual,
            limit: MAX_REVISION3_ENTITY_JSON_BYTES,
        }) if actual == MAX_REVISION3_ENTITY_JSON_BYTES + 1
    ));
}

#[test]
fn plan_binding_includes_exact_project_revision_and_bytes() {
    let project = project();
    let canonical = project.to_canonical_json().unwrap();
    let plan =
        build_revision3_npc_source_inspection_plan_v1(&canonical, entity_id(NPC_ID)).unwrap();

    let mut changed = project;
    changed.revision += 1;
    let changed_canonical = changed.to_canonical_json().unwrap();
    assert!(matches!(
        plan.verify_against_project(&changed_canonical),
        Err(Revision3NpcInspectionErrorV1::PlanProjectBindingMismatch)
    ));
}
