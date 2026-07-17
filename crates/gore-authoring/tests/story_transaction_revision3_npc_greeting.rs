use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::model_revision3::{DialogLine, LocalizationEntry};
use gore_authoring::{
    apply_revision3_npc_greeting_edit_transaction_v1, build_revision3_content_index_v1,
    migrate_revision2_to_revision3, AssetStoreIndex, ContentSeal, EntityId, FormatV2,
    GameGenerationAnchor, ProjectId, ProjectMeta, ProjectRevision2, ProjectRevision3,
    ProjectRevision3ValidationError, Revision2Entity, Revision2EntityPayload, Revision2NpcDraft,
    Revision2NpcDraftInput, Revision2NpcParentClassInput, Revision2OriginRef,
    Revision3ContentEntitySummaryV1, Revision3ContentReferenceRoleV1,
    Revision3DialogEmptyVoiceSlotIntentV1, Revision3DialogLineInsertRequestV1,
    Revision3DialogLocalizationActionV1, Revision3DialogLocalizationIntentV1, Revision3Entity,
    Revision3EntityKind, Revision3EntityPayload, Revision3NpcDraft, Revision3NpcGreetingBindingV1,
    Revision3NpcGreetingBuildStatusV1, Revision3NpcGreetingEditConflictV1,
    Revision3NpcGreetingEditEvaluationV1, Revision3NpcGreetingEditOutcomeV1,
    Revision3NpcGreetingEditRequestJsonErrorV1, Revision3NpcGreetingEditRequestV1,
    Revision3NpcGreetingIntentV1, Revision3NpcGreetingModeV1,
    Revision3NpcGreetingPublicationStatusV1, Revision3NpcGreetingRuntimeStatusV1,
    Revision3NpcGreetingTopicAuthorityV1, Revision3OriginRef, Revision3TypedRef, SchemaRevisionV2,
    SchemaRevisionV3, Sha256Digest, WorkingHead, WorkingStoreFormat,
    LOGICAL_NPC_CLONE_GENERATOR_ID, LOGICAL_NPC_CLONE_GENERATOR_VERSION,
    MAX_REVISION3_NPC_GREETING_BINDINGS_V1, MAX_REVISION3_NPC_GREETING_REQUEST_JSON_BYTES_V1,
};

const NPC: u8 = 0x21;
const MODULE: u8 = 0x22;
const LINE_A: u8 = 0x31;
const LOC_A: u8 = 0x32;
const LINE_B: u8 = 0x33;
const LOC_B: u8 = 0x34;
const UNRELATED: u8 = 0x40;

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
        snapshot: seal(value, 4096),
    }
}

fn locale(value: &str) -> gore_authoring::LocaleCode {
    value.parse().unwrap()
}

fn parent(
    generation: &GameGenerationAnchor,
    value: u8,
    runtime_class: &str,
) -> Revision2NpcParentClassInput {
    Revision2NpcParentClassInput {
        generation: generation.clone(),
        source_seal: seal(value, 4096),
        catalog_layer: "base-game.g1r.npcs".to_owned(),
        canonical_selector: format!("Catalog_{runtime_class}"),
        runtime_class: runtime_class.to_owned(),
    }
}

fn dialog_entities(
    project: ProjectId,
    line_id: EntityId,
    localization_id: EntityId,
    suffix: &str,
) -> [(EntityId, Revision3Entity); 2] {
    [
        (
            localization_id,
            Revision3Entity {
                id: localization_id,
                display_name: format!("Greeting {suffix} localization"),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: format!("GORE_GREETING_{suffix}_LOC"),
                },
                revision: 2,
                payload: Revision3EntityPayload::LocalizationEntry(LocalizationEntry {
                    loc_id: format!("GORE_GREETING_{suffix}"),
                    texts: BTreeMap::new(),
                }),
            },
        ),
        (
            line_id,
            Revision3Entity {
                id: line_id,
                display_name: format!("Greeting {suffix}"),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: format!("GORE_GREETING_{suffix}"),
                },
                revision: 3,
                payload: Revision3EntityPayload::DialogLine(DialogLine {
                    localization: Revision3TypedRef::new(
                        project,
                        localization_id,
                        Revision3EntityKind::LocalizationEntry,
                    ),
                    speaker_hint: Some("Asghan".to_owned()),
                    voice_slots: BTreeMap::new(),
                }),
            },
        ),
    ]
}

fn project() -> (ProjectRevision3, WorkingHead) {
    let project_id = project_id(0x11);
    let generation = target(0x12);
    let npc_id = id(NPC);
    let module_id = id(MODULE);
    let owner = Revision3TypedRef::new(project_id, npc_id, Revision3EntityKind::NpcDraft);
    let npc = Revision3NpcDraft {
        generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
        generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
        input: Revision2NpcDraftInput {
            target: generation.clone(),
            module_namespace: "GoreMods.Npcs.GreetingGuard".to_owned(),
            unique_name: "GORE_GREETING_GUARD".to_owned(),
            parent_character_definition: parent(
                &generation,
                0x41,
                "UCharacterDefinition_Human_OM_GRD_Asghan_263",
            ),
            parent_ai_agent_config: parent(
                &generation,
                0x42,
                "UAIAgentConfig_Human_OM_GRD_Asghan_263",
            ),
            parent_spawn_definition: parent(
                &generation,
                0x43,
                "USpawnAIAgentDefinition_OM_GRD_Asghan_263",
            ),
        },
        script_module: Revision3TypedRef::new(
            project_id,
            module_id,
            Revision3EntityKind::ScriptModule,
        ),
        greetings: Vec::new(),
    };
    let module = npc.regenerate_script_module(owner.clone()).unwrap();
    let mut entities = BTreeMap::from([
        (
            npc_id,
            Revision3Entity {
                id: npc_id,
                display_name: "Greeting guard".to_owned(),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: "GORE_GREETING_GUARD".to_owned(),
                },
                revision: 3,
                payload: Revision3EntityPayload::NpcDraft(npc),
            },
        ),
        (
            module_id,
            Revision3Entity {
                id: module_id,
                display_name: "Greeting guard source".to_owned(),
                origin: Revision3OriginRef::Generated {
                    generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
                    generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
                    owner,
                },
                revision: 5,
                payload: Revision3EntityPayload::ScriptModule(module),
            },
        ),
        (
            id(UNRELATED),
            Revision3Entity {
                id: id(UNRELATED),
                display_name: "Unrelated".to_owned(),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: "GORE_UNRELATED".to_owned(),
                },
                revision: 9,
                payload: Revision3EntityPayload::LocalizationEntry(LocalizationEntry {
                    loc_id: "GORE_UNRELATED".to_owned(),
                    texts: BTreeMap::new(),
                }),
            },
        ),
    ]);
    entities.extend(dialog_entities(project_id, id(LINE_A), id(LOC_A), "A"));
    entities.extend(dialog_entities(project_id, id(LINE_B), id(LOC_B), "B"));
    let project = ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id,
        revision: 7,
        meta: ProjectMeta {
            name: "NPC greeting tests".to_owned(),
            version: "1.0.0".to_owned(),
            author: "tests".to_owned(),
        },
        target: generation,
        authoring_locales: BTreeSet::new(),
        entities,
        asset_store: AssetStoreIndex {
            assets: BTreeMap::new(),
        },
    };
    project.validate_closed_model().unwrap();
    (project, head(0x18))
}

fn binding(project: &ProjectRevision3, line: u8) -> Revision3NpcGreetingBindingV1 {
    Revision3NpcGreetingBindingV1 {
        line: Revision3TypedRef::new(
            project.project_id,
            id(line),
            Revision3EntityKind::DialogLine,
        ),
    }
}

fn npc(project: &ProjectRevision3) -> &Revision3NpcDraft {
    let Revision3EntityPayload::NpcDraft(npc) = &project.entities[&id(NPC)].payload else {
        panic!("fixture NPC kind")
    };
    npc
}

fn request(
    project: &ProjectRevision3,
    basis_head: &WorkingHead,
    intent: Revision3NpcGreetingIntentV1,
) -> Revision3NpcGreetingEditRequestV1 {
    Revision3NpcGreetingEditRequestV1 {
        expected_head: basis_head.clone(),
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        expected_target: project.target.clone(),
        npc_id: id(NPC),
        expected_npc_revision: project.entities[&id(NPC)].revision,
        intent,
    }
}

fn evaluate(
    project: &ProjectRevision3,
    basis_head: &WorkingHead,
    request: &Revision3NpcGreetingEditRequestV1,
) -> Revision3NpcGreetingEditEvaluationV1 {
    apply_revision3_npc_greeting_edit_transaction_v1(
        basis_head,
        &project.to_canonical_json().unwrap(),
        &request.to_canonical_json().unwrap(),
    )
    .unwrap()
}

fn applied(value: Revision3NpcGreetingEditEvaluationV1) -> Revision3NpcGreetingEditOutcomeV1 {
    match value {
        Revision3NpcGreetingEditEvaluationV1::Applied(outcome) => *outcome,
        Revision3NpcGreetingEditEvaluationV1::Rejected(rejection) => {
            panic!("unexpected rejection: {}", rejection.conflict)
        }
    }
}

fn rejected(value: Revision3NpcGreetingEditEvaluationV1) -> Revision3NpcGreetingEditConflictV1 {
    match value {
        Revision3NpcGreetingEditEvaluationV1::Rejected(rejection) => rejection.conflict,
        Revision3NpcGreetingEditEvaluationV1::Applied(_) => panic!("unexpected candidate"),
    }
}

#[test]
fn empty_greetings_preserve_pre_feature_npc_and_project_canonical_bytes() {
    let (project, _) = project();
    let r3 = npc(&project);
    let r2 = Revision2NpcDraft {
        generator_id: r3.generator_id.clone(),
        generator_version: r3.generator_version,
        input: r3.input.clone(),
        script_module: r3.script_module.clone(),
    };
    assert_eq!(
        serde_json::to_vec(r3).unwrap(),
        serde_json::to_vec(&r2).unwrap()
    );

    let json = project.to_canonical_json().unwrap();
    assert!(!json.contains("\"greetings\""));
    assert_eq!(ProjectRevision3::from_json(&json).unwrap(), project);
    assert_eq!(project.to_canonical_json().unwrap(), json);
}

#[test]
fn revision2_npc_migration_produces_empty_r3_greetings_without_wire_drift() {
    let (project, _) = project();
    let r3_npc = npc(&project);
    let npc_id = id(NPC);
    let module_id = id(MODULE);
    let r2_npc = Revision2NpcDraft {
        generator_id: r3_npc.generator_id.clone(),
        generator_version: r3_npc.generator_version,
        input: r3_npc.input.clone(),
        script_module: r3_npc.script_module.clone(),
    };
    let source = ProjectRevision2 {
        format: FormatV2,
        schema_revision: SchemaRevisionV2,
        project_id: project.project_id,
        revision: project.revision,
        meta: project.meta.clone(),
        target: project.target.clone(),
        authoring_locales: project.authoring_locales.clone(),
        entities: BTreeMap::from([
            (
                npc_id,
                Revision2Entity {
                    id: npc_id,
                    display_name: project.entities[&npc_id].display_name.clone(),
                    origin: Revision2OriginRef::New {
                        authored_runtime_id: r3_npc.input.unique_name.clone(),
                    },
                    revision: project.entities[&npc_id].revision,
                    payload: Revision2EntityPayload::NpcDraft(r2_npc.clone()),
                },
            ),
            (
                module_id,
                Revision2Entity {
                    id: module_id,
                    display_name: project.entities[&module_id].display_name.clone(),
                    origin: project.entities[&module_id].origin.clone(),
                    revision: project.entities[&module_id].revision,
                    payload: match &project.entities[&module_id].payload {
                        Revision3EntityPayload::ScriptModule(module) => {
                            Revision2EntityPayload::ScriptModule(module.clone())
                        }
                        _ => unreachable!(),
                    },
                },
            ),
        ]),
        asset_store: AssetStoreIndex::default(),
    };
    source.to_canonical_json().unwrap();

    let migrated = migrate_revision2_to_revision3(&source).unwrap();
    let Revision3EntityPayload::NpcDraft(migrated_npc) =
        &migrated.project.entities[&npc_id].payload
    else {
        unreachable!()
    };
    assert!(migrated_npc.greetings.is_empty());
    assert_eq!(
        serde_json::to_vec(migrated_npc).unwrap(),
        serde_json::to_vec(&r2_npc).unwrap()
    );
    assert!(!migrated
        .project
        .to_canonical_json()
        .unwrap()
        .contains("\"greetings\""));
}

#[test]
fn request_wire_is_exact_closed_duplicate_free_and_bounded() {
    let (project, basis_head) = project();
    let request = request(
        &project,
        &basis_head,
        Revision3NpcGreetingIntentV1::Replace {
            bindings: vec![binding(&project, LINE_A)],
        },
    );
    let canonical = request.to_canonical_json().unwrap();
    assert!(canonical.starts_with("{\"expected_head\":"));
    assert!(canonical.contains("\"intent\":{\"mode\":\"replace\",\"bindings\":[{\"line\":"));
    assert_eq!(
        Revision3NpcGreetingEditRequestV1::from_json(&canonical).unwrap(),
        request
    );
    assert!(matches!(
        Revision3NpcGreetingEditRequestV1::from_json(&(canonical.clone() + "\n")),
        Err(Revision3NpcGreetingEditRequestJsonErrorV1::NonCanonicalJson)
    ));
    let duplicate = canonical.replacen(
        "\"expected_revision\":7,",
        "\"expected_revision\":7,\"expected_revision\":7,",
        1,
    );
    assert!(matches!(
        Revision3NpcGreetingEditRequestV1::from_json(&duplicate),
        Err(Revision3NpcGreetingEditRequestJsonErrorV1::InvalidJson(_))
    ));
    let unknown = canonical.replacen("\"intent\":", "\"game_root\":\"C:/game\",\"intent\":", 1);
    assert!(matches!(
        Revision3NpcGreetingEditRequestV1::from_json(&unknown),
        Err(Revision3NpcGreetingEditRequestJsonErrorV1::InvalidJson(_))
    ));
    assert!(matches!(
        Revision3NpcGreetingEditRequestV1::from_json(
            &" ".repeat(MAX_REVISION3_NPC_GREETING_REQUEST_JSON_BYTES_V1 + 1)
        ),
        Err(Revision3NpcGreetingEditRequestJsonErrorV1::InputTooLarge { .. })
    ));
}

#[test]
fn replace_reorders_detaches_and_indexes_order_without_touching_generation() {
    let (project, basis_head) = project();
    let basis_npc = project.entities[&id(NPC)].clone();
    let basis_module = project.entities[&id(MODULE)].clone();
    let basis_unrelated = project.entities[&id(UNRELATED)].clone();
    let basis_input_json = serde_json::to_vec(&npc(&project).input).unwrap();
    let bindings = vec![binding(&project, LINE_B), binding(&project, LINE_A)];
    let edit = request(
        &project,
        &basis_head,
        Revision3NpcGreetingIntentV1::Replace {
            bindings: bindings.clone(),
        },
    );
    let outcome = applied(evaluate(&project, &basis_head, &edit));

    assert_eq!(outcome.mode, Revision3NpcGreetingModeV1::Replace);
    assert_eq!(outcome.project.revision, project.revision + 1);
    assert_eq!(outcome.npc_revision, basis_npc.revision + 1);
    assert_eq!(outcome.script_module_revision, basis_module.revision);
    assert_eq!(outcome.greeting_count, 2);
    assert!(outcome.created.is_none());
    assert_eq!(npc(&outcome.project).greetings, bindings);
    assert_eq!(
        serde_json::to_vec(&npc(&outcome.project).input).unwrap(),
        basis_input_json
    );
    assert_eq!(outcome.project.entities[&id(MODULE)], basis_module);
    assert_eq!(outcome.project.entities[&id(UNRELATED)], basis_unrelated);
    assert_eq!(outcome.project.asset_store, project.asset_store);
    assert_eq!(
        outcome.build_status,
        Revision3NpcGreetingBuildStatusV1::Blocked
    );
    assert_eq!(
        outcome.runtime_status,
        Revision3NpcGreetingRuntimeStatusV1::RuntimeUnqualified
    );
    assert_eq!(
        outcome.topic_authority,
        Revision3NpcGreetingTopicAuthorityV1::NotGranted
    );
    assert_eq!(
        outcome.publication_status,
        Revision3NpcGreetingPublicationStatusV1::NotSupported
    );

    let index = build_revision3_content_index_v1(&outcome.project).unwrap();
    let indexed_npc = index
        .entities
        .iter()
        .find(|entity| entity.id == id(NPC))
        .unwrap();
    assert_eq!(indexed_npc.references.len(), 3);
    assert_eq!(
        indexed_npc.references[0].role,
        Revision3ContentReferenceRoleV1::DraftScriptModule
    );
    assert_eq!(
        indexed_npc.references[1].role,
        Revision3ContentReferenceRoleV1::NpcGreetingLine
    );
    assert_eq!(indexed_npc.references[1].target.entity_id, id(LINE_B));
    assert_eq!(indexed_npc.references[1].qualifier, None);
    assert_eq!(indexed_npc.references[2].target.entity_id, id(LINE_A));
    let Revision3ContentEntitySummaryV1::NpcDraft { greeting_count, .. } = &indexed_npc.summary
    else {
        panic!("NPC summary kind")
    };
    assert_eq!(*greeting_count, 2);

    let no_change = request(
        &outcome.project,
        &basis_head,
        Revision3NpcGreetingIntentV1::Replace {
            bindings: npc(&outcome.project).greetings.clone(),
        },
    );
    assert!(matches!(
        rejected(evaluate(&outcome.project, &basis_head, &no_change)),
        Revision3NpcGreetingEditConflictV1::NoChanges
    ));

    let detach = request(
        &outcome.project,
        &basis_head,
        Revision3NpcGreetingIntentV1::Replace { bindings: vec![] },
    );
    let detached = applied(evaluate(&outcome.project, &basis_head, &detach));
    assert!(npc(&detached.project).greetings.is_empty());
    assert!(!detached.canonical_project_json.contains("\"greetings\""));
    assert!(detached.project.entities.contains_key(&id(LINE_A)));
    assert!(detached.project.entities.contains_key(&id(LINE_B)));
}

#[test]
fn create_and_insert_is_one_atomic_project_and_npc_revision() {
    let (mut project, basis_head) = project();
    let existing = binding(&project, LINE_A);
    let Revision3EntityPayload::NpcDraft(npc_payload) =
        &mut project.entities.get_mut(&id(NPC)).unwrap().payload
    else {
        panic!("fixture NPC kind")
    };
    npc_payload.greetings = vec![existing.clone()];
    project.validate_closed_model().unwrap();
    let basis = project.clone();

    let dialog = Revision3DialogLineInsertRequestV1 {
        expected_head: basis_head.clone(),
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        expected_target: project.target.clone(),
        line_id: id(0x51),
        line_display_name: "Created NPC greeting".to_owned(),
        line_authored_identity: "GORE_CREATED_NPC_GREETING".to_owned(),
        speaker_hint: Some("Asghan".to_owned()),
        localization: Revision3DialogLocalizationIntentV1::Create {
            localization_id: id(0x52),
            display_name: "Created NPC greeting localization".to_owned(),
            loc_id: "GORE_CREATED_NPC_GREETING_TEXT".to_owned(),
            texts: BTreeMap::from([
                (locale("de"), "Willkommen.".to_owned()),
                (locale("en"), "Welcome.".to_owned()),
            ]),
        },
        voice_slot: None::<Revision3DialogEmptyVoiceSlotIntentV1>,
    };
    let edit = request(
        &project,
        &basis_head,
        Revision3NpcGreetingIntentV1::CreateAndInsert {
            index: 1,
            line: dialog,
        },
    );
    let outcome = applied(evaluate(&project, &basis_head, &edit));

    assert_eq!(outcome.project.revision, basis.revision + 1);
    assert_eq!(outcome.npc_revision, basis.entities[&id(NPC)].revision + 1);
    assert_eq!(
        outcome.script_module_revision,
        basis.entities[&id(MODULE)].revision
    );
    assert_eq!(outcome.mode, Revision3NpcGreetingModeV1::CreateAndInsert);
    assert_eq!(outcome.greeting_count, 2);
    let created = outcome.created.unwrap();
    assert_eq!(created.line_id, id(0x51));
    assert_eq!(created.localization_id, id(0x52));
    assert_eq!(created.voice_slot_id, None);
    assert_eq!(
        created.localization_action,
        Revision3DialogLocalizationActionV1::Created
    );
    assert_eq!(
        npc(&outcome.project).greetings,
        vec![
            existing,
            Revision3NpcGreetingBindingV1 {
                line: Revision3TypedRef::new(
                    project.project_id,
                    id(0x51),
                    Revision3EntityKind::DialogLine,
                ),
            },
        ]
    );
    assert_eq!(
        outcome.project.entities[&id(MODULE)],
        basis.entities[&id(MODULE)]
    );
    assert_eq!(npc(&outcome.project).input, npc(&basis).input);
    assert_eq!(outcome.project.entities.len(), basis.entities.len() + 2);
}

#[test]
fn conflicts_cover_cas_binding_closure_caps_and_embedded_dialog_rejection() {
    let (project, basis_head) = project();

    let mut stale = request(
        &project,
        &basis_head,
        Revision3NpcGreetingIntentV1::Replace {
            bindings: vec![binding(&project, LINE_A)],
        },
    );
    stale.expected_npc_revision += 1;
    assert!(matches!(
        rejected(evaluate(&project, &basis_head, &stale)),
        Revision3NpcGreetingEditConflictV1::NpcRevisionConflict { .. }
    ));

    let duplicate = request(
        &project,
        &basis_head,
        Revision3NpcGreetingIntentV1::Replace {
            bindings: vec![binding(&project, LINE_A), binding(&project, LINE_A)],
        },
    );
    assert!(matches!(
        rejected(evaluate(&project, &basis_head, &duplicate)),
        Revision3NpcGreetingEditConflictV1::DuplicateLine { .. }
    ));

    let mut foreign = binding(&project, LINE_A);
    foreign.line.project_id = project_id(0xfe);
    let foreign = request(
        &project,
        &basis_head,
        Revision3NpcGreetingIntentV1::Replace {
            bindings: vec![foreign],
        },
    );
    assert!(matches!(
        rejected(evaluate(&project, &basis_head, &foreign)),
        Revision3NpcGreetingEditConflictV1::InvalidLineReference { .. }
    ));

    let too_many = request(
        &project,
        &basis_head,
        Revision3NpcGreetingIntentV1::Replace {
            bindings: vec![binding(&project, LINE_A); MAX_REVISION3_NPC_GREETING_BINDINGS_V1 + 1],
        },
    );
    assert!(matches!(
        rejected(evaluate(&project, &basis_head, &too_many)),
        Revision3NpcGreetingEditConflictV1::TooManyBindings { .. }
    ));

    let embedded_duplicate = Revision3DialogLineInsertRequestV1 {
        expected_head: basis_head.clone(),
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        expected_target: project.target.clone(),
        line_id: id(LINE_A),
        line_display_name: "Duplicate".to_owned(),
        line_authored_identity: "GORE_DUPLICATE".to_owned(),
        speaker_hint: None,
        localization: Revision3DialogLocalizationIntentV1::Create {
            localization_id: id(0x61),
            display_name: "Duplicate loc".to_owned(),
            loc_id: "GORE_DUPLICATE".to_owned(),
            texts: BTreeMap::new(),
        },
        voice_slot: None,
    };
    let create = request(
        &project,
        &basis_head,
        Revision3NpcGreetingIntentV1::CreateAndInsert {
            index: 0,
            line: embedded_duplicate,
        },
    );
    assert!(matches!(
        rejected(evaluate(&project, &basis_head, &create)),
        Revision3NpcGreetingEditConflictV1::DialogLineRejected { .. }
    ));
    assert_eq!(project.revision, 7);
    assert_eq!(project.entities.len(), 7);
}

#[test]
fn closed_model_rejects_foreign_wrong_kind_missing_duplicate_and_over_cap_bindings() {
    let (project, _) = project();
    let assert_invalid = |mut candidate: ProjectRevision3| {
        assert!(matches!(
            candidate.validate_closed_model(),
            Err(ProjectRevision3ValidationError::InvalidNpcGreetings { .. })
        ));
        // Keep the mutable binding explicit so every caller starts from an independent project.
        candidate.revision = candidate.revision.saturating_add(0);
    };

    let mut foreign = project.clone();
    let Revision3EntityPayload::NpcDraft(value) =
        &mut foreign.entities.get_mut(&id(NPC)).unwrap().payload
    else {
        unreachable!()
    };
    value.greetings = vec![binding(&project, LINE_A)];
    value.greetings[0].line.project_id = project_id(0xfe);
    assert_invalid(foreign);

    let mut wrong_kind = project.clone();
    let Revision3EntityPayload::NpcDraft(value) =
        &mut wrong_kind.entities.get_mut(&id(NPC)).unwrap().payload
    else {
        unreachable!()
    };
    value.greetings = vec![binding(&project, LINE_A)];
    value.greetings[0].line.expected_kind = Revision3EntityKind::LocalizationEntry;
    assert_invalid(wrong_kind);

    let mut missing = project.clone();
    let Revision3EntityPayload::NpcDraft(value) =
        &mut missing.entities.get_mut(&id(NPC)).unwrap().payload
    else {
        unreachable!()
    };
    value.greetings = vec![Revision3NpcGreetingBindingV1 {
        line: Revision3TypedRef::new(
            project.project_id,
            id(0xee),
            Revision3EntityKind::DialogLine,
        ),
    }];
    assert_invalid(missing);

    let mut duplicate = project.clone();
    let Revision3EntityPayload::NpcDraft(value) =
        &mut duplicate.entities.get_mut(&id(NPC)).unwrap().payload
    else {
        unreachable!()
    };
    value.greetings = vec![binding(&project, LINE_A), binding(&project, LINE_A)];
    assert_invalid(duplicate);

    let mut too_many = project.clone();
    let Revision3EntityPayload::NpcDraft(value) =
        &mut too_many.entities.get_mut(&id(NPC)).unwrap().payload
    else {
        unreachable!()
    };
    value.greetings = vec![binding(&project, LINE_A); MAX_REVISION3_NPC_GREETING_BINDINGS_V1 + 1];
    assert_invalid(too_many);
}
