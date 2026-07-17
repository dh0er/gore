use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::model_revision3::{DialogLine, LocalizationEntry};
use gore_authoring::{
    apply_revision3_quest_transcript_edit_transaction_v1, build_revision3_content_index_v1,
    regenerate_revision3_quest_module_v2, AssetMeta, AssetStoreIndex, ContentSeal, EntityId,
    FormatV2, GameGenerationAnchor, LocaleCode, ProjectId, ProjectMeta, ProjectRevision3,
    QuestCollisionArtifactRef, QuestCollisionCatalogInput, QuestTransitionPlanV1,
    Revision3ContentEntitySummaryV1, Revision3ContentReferenceRoleV1,
    Revision3DialogEmptyVoiceSlotIntentV1, Revision3DialogLineInsertRequestV1,
    Revision3DialogLocalizationActionV1, Revision3DialogLocalizationIntentV1, Revision3Entity,
    Revision3EntityKind, Revision3EntityPayload, Revision3OriginRef, Revision3QuestDraft,
    Revision3QuestDraftInput, Revision3QuestGiverInput, Revision3QuestParentInput,
    Revision3QuestTranscriptBindingV1, Revision3QuestTranscriptEditConflictV1,
    Revision3QuestTranscriptEditEvaluationV1, Revision3QuestTranscriptEditOutcomeV1,
    Revision3QuestTranscriptEditRequestJsonErrorV1, Revision3QuestTranscriptEditRequestV1,
    Revision3QuestTranscriptIntentV1, Revision3QuestTranscriptModeV1, Revision3ScriptModule,
    Revision3TypedRef, SchemaRevisionV3, Sha256Digest, WorkingHead, WorkingStoreFormat,
    MAX_REVISION3_QUEST_TRANSCRIPT_BINDINGS_V1,
    MAX_REVISION3_QUEST_TRANSCRIPT_REQUEST_JSON_BYTES_V1, QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2,
    QUEST_COLLISION_CATALOG_LAYER_V2, REVISION3_QUEST_GENERATOR_ID,
    REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION,
};

const QUEST: u8 = 0x21;
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

fn locale(value: &str) -> LocaleCode {
    value.parse().unwrap()
}

fn collision_input(quest: &Revision3QuestDraft) -> QuestCollisionCatalogInput {
    QuestCollisionCatalogInput {
        generation: quest.input.collision_catalog.generation.clone(),
        source_seal: quest.input.collision_catalog.source_seal.clone(),
        catalog_layer: quest.input.collision_catalog.catalog_layer.clone(),
        modules: BTreeSet::new(),
        relative_paths: BTreeSet::new(),
        symbols: BTreeSet::new(),
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
                display_name: format!("Line {suffix} localization"),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: format!("GORE_LINE_{suffix}_LOC"),
                },
                revision: 2,
                payload: Revision3EntityPayload::LocalizationEntry(LocalizationEntry {
                    loc_id: format!("GORE_LINE_{suffix}"),
                    texts: BTreeMap::new(),
                }),
            },
        ),
        (
            line_id,
            Revision3Entity {
                id: line_id,
                display_name: format!("Line {suffix}"),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: format!("GORE_LINE_{suffix}"),
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
    let artifact = seal(0x13, 8192);
    let quest_id = id(QUEST);
    let module_id = id(MODULE);
    let quest = Revision3QuestDraft {
        generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
        generator_version: REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION,
        input: Revision3QuestDraftInput {
            target: generation.clone(),
            quest_id,
            module_namespace: "GoreMods.Quests.Transcript".to_owned(),
            technical_id: "GORE_QUEST_TRANSCRIPT".to_owned(),
            text_helper: "GoreQuestTranscriptText".to_owned(),
            parent_quest: Revision3QuestParentInput {
                generation: generation.clone(),
                source_seal: seal(0x14, 100),
                catalog_layer: "base-game.g1r.quests".to_owned(),
                canonical_selector: "CatalogQuest_Parent".to_owned(),
                runtime_class: "UQuest_Parent".to_owned(),
            },
            giver: Revision3QuestGiverInput {
                generation: generation.clone(),
                source_seal: seal(0x15, 100),
                catalog_layer: "base-game.g1r.characters".to_owned(),
                canonical_selector: "CatalogCharacter_Asghan".to_owned(),
                runtime_unique_name: "OM_GRD_Asghan_263".to_owned(),
            },
            title: "Transcript Quest".to_owned(),
            description: "Authoring-only dialog order".to_owned(),
            objective_title: "First".to_owned(),
            additional_objective_titles: vec!["Second".to_owned()],
            transition_plan: Some(Box::new(QuestTransitionPlanV1::legacy_seed(2).unwrap())),
            collision_catalog: QuestCollisionArtifactRef {
                generation: generation.clone(),
                catalog_layer: QUEST_COLLISION_CATALOG_LAYER_V2.to_owned(),
                artifact: artifact.clone(),
                source_seal: seal(0x16, artifact.byte_len),
                basis_snapshot: seal(0x17, 4096),
            },
        },
        script_module: Revision3TypedRef::new(
            project_id,
            module_id,
            Revision3EntityKind::ScriptModule,
        ),
        transcript: Vec::new(),
    };
    let module = regenerate_revision3_quest_module_v2(&quest, collision_input(&quest)).unwrap();
    let owner = Revision3TypedRef::new(project_id, quest_id, Revision3EntityKind::QuestDraft);
    let mut entities = BTreeMap::from([
        (
            quest_id,
            Revision3Entity {
                id: quest_id,
                display_name: "Transcript Quest".to_owned(),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: "GORE_QUEST_TRANSCRIPT".to_owned(),
                },
                revision: 3,
                payload: Revision3EntityPayload::QuestDraft(quest),
            },
        ),
        (
            module_id,
            Revision3Entity {
                id: module_id,
                display_name: "Transcript Quest module".to_owned(),
                origin: Revision3OriginRef::Generated {
                    generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
                    generator_version: REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION,
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
            name: "Quest transcript tests".to_owned(),
            version: "1.0.0".to_owned(),
            author: "tests".to_owned(),
        },
        target: generation,
        authoring_locales: BTreeSet::new(),
        entities,
        asset_store: AssetStoreIndex {
            assets: BTreeMap::from([(
                artifact.sha256,
                AssetMeta {
                    byte_len: artifact.byte_len,
                    media_type: QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2.to_owned(),
                },
            )]),
        },
    };
    project.validate_closed_model().unwrap();
    (project, head(0x18))
}

fn binding(
    project: &ProjectRevision3,
    line: u8,
    slot: Option<u16>,
) -> Revision3QuestTranscriptBindingV1 {
    Revision3QuestTranscriptBindingV1 {
        line: Revision3TypedRef::new(
            project.project_id,
            id(line),
            Revision3EntityKind::DialogLine,
        ),
        objective_slot: slot,
    }
}

fn request(
    project: &ProjectRevision3,
    basis_head: &WorkingHead,
    intent: Revision3QuestTranscriptIntentV1,
) -> Revision3QuestTranscriptEditRequestV1 {
    Revision3QuestTranscriptEditRequestV1 {
        expected_head: basis_head.clone(),
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        expected_target: project.target.clone(),
        quest_id: id(QUEST),
        expected_quest_revision: project.entities[&id(QUEST)].revision,
        intent,
    }
}

fn evaluate(
    project: &ProjectRevision3,
    basis_head: &WorkingHead,
    request: &Revision3QuestTranscriptEditRequestV1,
) -> Revision3QuestTranscriptEditEvaluationV1 {
    apply_revision3_quest_transcript_edit_transaction_v1(
        basis_head,
        &project.to_canonical_json().unwrap(),
        &request.to_canonical_json().unwrap(),
    )
    .unwrap()
}

fn applied(
    value: Revision3QuestTranscriptEditEvaluationV1,
) -> Revision3QuestTranscriptEditOutcomeV1 {
    match value {
        Revision3QuestTranscriptEditEvaluationV1::Applied(outcome) => *outcome,
        Revision3QuestTranscriptEditEvaluationV1::Rejected(rejection) => {
            panic!("unexpected rejection: {}", rejection.conflict)
        }
    }
}

fn rejected(
    value: Revision3QuestTranscriptEditEvaluationV1,
) -> Revision3QuestTranscriptEditConflictV1 {
    match value {
        Revision3QuestTranscriptEditEvaluationV1::Rejected(rejection) => rejection.conflict,
        Revision3QuestTranscriptEditEvaluationV1::Applied(_) => panic!("unexpected candidate"),
    }
}

fn quest(project: &ProjectRevision3) -> &Revision3QuestDraft {
    let Revision3EntityPayload::QuestDraft(quest) = &project.entities[&id(QUEST)].payload else {
        panic!("fixture Quest kind")
    };
    quest
}

#[test]
fn empty_transcript_preserves_pre_feature_canonical_bytes() {
    let (project, _) = project();
    let json = project.to_canonical_json().unwrap();
    assert!(!json.contains("\"transcript\""));
    assert_eq!(ProjectRevision3::from_json(&json).unwrap(), project);
    assert_eq!(project.to_canonical_json().unwrap(), json);
}

#[test]
fn request_wire_is_exact_closed_duplicate_free_bounded_and_spells_null_slots() {
    let (project, basis_head) = project();
    let request = request(
        &project,
        &basis_head,
        Revision3QuestTranscriptIntentV1::Replace {
            bindings: vec![binding(&project, LINE_A, None)],
        },
    );
    let canonical = request.to_canonical_json().unwrap();
    assert!(canonical.starts_with("{\"expected_head\":"));
    assert!(canonical.contains("\"intent\":{\"mode\":\"replace\",\"bindings\":["));
    assert!(canonical.contains("\"objective_slot\":null"));
    assert_eq!(
        Revision3QuestTranscriptEditRequestV1::from_json(&canonical).unwrap(),
        request
    );
    assert!(matches!(
        Revision3QuestTranscriptEditRequestV1::from_json(&(canonical.clone() + "\n")),
        Err(Revision3QuestTranscriptEditRequestJsonErrorV1::NonCanonicalJson)
    ));

    let duplicate = canonical.replacen(
        "\"expected_revision\":7,",
        "\"expected_revision\":7,\"expected_revision\":7,",
        1,
    );
    assert!(matches!(
        Revision3QuestTranscriptEditRequestV1::from_json(&duplicate),
        Err(Revision3QuestTranscriptEditRequestJsonErrorV1::InvalidJson(
            _
        ))
    ));
    let unknown = canonical.replacen("\"intent\":", "\"game_root\":\"C:/game\",\"intent\":", 1);
    assert!(matches!(
        Revision3QuestTranscriptEditRequestV1::from_json(&unknown),
        Err(Revision3QuestTranscriptEditRequestJsonErrorV1::InvalidJson(
            _
        ))
    ));
    let missing_null = canonical.replacen(",\"objective_slot\":null", "", 1);
    assert!(matches!(
        Revision3QuestTranscriptEditRequestV1::from_json(&missing_null),
        Err(Revision3QuestTranscriptEditRequestJsonErrorV1::NonCanonicalJson)
    ));
    assert!(matches!(
        Revision3QuestTranscriptEditRequestV1::from_json(
            &" ".repeat(MAX_REVISION3_QUEST_TRANSCRIPT_REQUEST_JSON_BYTES_V1 + 1)
        ),
        Err(Revision3QuestTranscriptEditRequestJsonErrorV1::InputTooLarge { .. })
    ));
}

#[test]
fn replace_reorders_detaches_and_projects_ordered_index_without_touching_generation() {
    let (project, basis_head) = project();
    let basis_module = project.entities[&id(MODULE)].clone();
    let basis_input = quest(&project).input.clone();
    let basis_assets = project.asset_store.clone();
    let basis_unrelated = project.entities[&id(UNRELATED)].clone();
    let bindings = vec![
        binding(&project, LINE_B, Some(2)),
        binding(&project, LINE_A, None),
    ];
    let edit = request(
        &project,
        &basis_head,
        Revision3QuestTranscriptIntentV1::Replace {
            bindings: bindings.clone(),
        },
    );
    let outcome = applied(evaluate(&project, &basis_head, &edit));

    assert_eq!(outcome.mode, Revision3QuestTranscriptModeV1::Replace);
    assert_eq!(outcome.project.revision, project.revision + 1);
    assert_eq!(
        outcome.quest_revision,
        project.entities[&id(QUEST)].revision + 1
    );
    assert_eq!(outcome.script_module_revision, basis_module.revision);
    assert_eq!(outcome.transcript_count, 2);
    assert!(outcome.created.is_none());
    assert_eq!(quest(&outcome.project).transcript, bindings);
    assert_eq!(quest(&outcome.project).input, basis_input);
    assert_eq!(outcome.project.entities[&id(MODULE)], basis_module);
    assert_eq!(outcome.project.entities[&id(UNRELATED)], basis_unrelated);
    assert_eq!(outcome.project.asset_store, basis_assets);

    let index = build_revision3_content_index_v1(&outcome.project).unwrap();
    let indexed_quest = index
        .entities
        .iter()
        .find(|entity| entity.id == id(QUEST))
        .unwrap();
    assert_eq!(indexed_quest.references.len(), 3);
    assert_eq!(
        indexed_quest.references[1].role,
        Revision3ContentReferenceRoleV1::QuestTranscriptLine
    );
    assert_eq!(indexed_quest.references[1].target.entity_id, id(LINE_B));
    assert_eq!(indexed_quest.references[1].qualifier.as_deref(), Some("2"));
    assert_eq!(indexed_quest.references[2].target.entity_id, id(LINE_A));
    assert_eq!(indexed_quest.references[2].qualifier, None);
    let Revision3ContentEntitySummaryV1::QuestDraft {
        objective_slots,
        transcript_count,
        ..
    } = &indexed_quest.summary
    else {
        panic!("Quest summary kind")
    };
    assert_eq!(objective_slots, &vec![1, 2]);
    assert_eq!(*transcript_count, 2);

    let no_change = request(
        &outcome.project,
        &basis_head,
        Revision3QuestTranscriptIntentV1::Replace {
            bindings: quest(&outcome.project).transcript.clone(),
        },
    );
    assert!(matches!(
        rejected(evaluate(&outcome.project, &basis_head, &no_change)),
        Revision3QuestTranscriptEditConflictV1::NoChanges
    ));

    let detach = request(
        &outcome.project,
        &basis_head,
        Revision3QuestTranscriptIntentV1::Replace { bindings: vec![] },
    );
    let detached = applied(evaluate(&outcome.project, &basis_head, &detach));
    assert!(quest(&detached.project).transcript.is_empty());
    assert!(!detached.canonical_project_json.contains("\"transcript\""));
    assert!(detached.project.entities.contains_key(&id(LINE_A)));
    assert!(detached.project.entities.contains_key(&id(LINE_B)));
}

#[test]
fn create_and_insert_is_one_atomic_project_and_quest_revision() {
    let (mut project, basis_head) = project();
    let existing = binding(&project, LINE_A, None);
    let Revision3EntityPayload::QuestDraft(quest_payload) =
        &mut project.entities.get_mut(&id(QUEST)).unwrap().payload
    else {
        panic!("fixture Quest kind")
    };
    quest_payload.transcript = vec![existing.clone()];
    project.validate_closed_model().unwrap();
    let basis = project.clone();

    let dialog = Revision3DialogLineInsertRequestV1 {
        expected_head: basis_head.clone(),
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        expected_target: project.target.clone(),
        line_id: id(0x51),
        line_display_name: "Created transcript line".to_owned(),
        line_authored_identity: "GORE_CREATED_TRANSCRIPT_LINE".to_owned(),
        speaker_hint: Some("Asghan".to_owned()),
        localization: Revision3DialogLocalizationIntentV1::Create {
            localization_id: id(0x52),
            display_name: "Created transcript localization".to_owned(),
            loc_id: "GORE_CREATED_TRANSCRIPT_TEXT".to_owned(),
            texts: BTreeMap::from([
                (locale("de"), "Eine neue Zeile.".to_owned()),
                (locale("en"), "A new line.".to_owned()),
            ]),
        },
        voice_slot: None::<Revision3DialogEmptyVoiceSlotIntentV1>,
    };
    let edit = request(
        &project,
        &basis_head,
        Revision3QuestTranscriptIntentV1::CreateAndInsert {
            index: 1,
            objective_slot: Some(1),
            line: dialog,
        },
    );
    let outcome = applied(evaluate(&project, &basis_head, &edit));

    assert_eq!(outcome.project.revision, basis.revision + 1);
    assert_eq!(
        outcome.quest_revision,
        basis.entities[&id(QUEST)].revision + 1
    );
    assert_eq!(
        outcome.script_module_revision,
        basis.entities[&id(MODULE)].revision
    );
    assert_eq!(
        outcome.mode,
        Revision3QuestTranscriptModeV1::CreateAndInsert
    );
    assert_eq!(outcome.transcript_count, 2);
    let created = outcome.created.unwrap();
    assert_eq!(created.line_id, id(0x51));
    assert_eq!(created.localization_id, id(0x52));
    assert_eq!(created.voice_slot_id, None);
    assert_eq!(
        created.localization_action,
        Revision3DialogLocalizationActionV1::Created
    );
    assert_eq!(
        quest(&outcome.project).transcript,
        vec![
            existing,
            Revision3QuestTranscriptBindingV1 {
                line: Revision3TypedRef::new(
                    project.project_id,
                    id(0x51),
                    Revision3EntityKind::DialogLine,
                ),
                objective_slot: Some(1),
            },
        ]
    );
    assert_eq!(
        outcome.project.entities[&id(MODULE)],
        basis.entities[&id(MODULE)]
    );
    assert_eq!(quest(&outcome.project).input, quest(&basis).input);
    assert_eq!(outcome.project.asset_store, basis.asset_store);
    assert_eq!(
        outcome.project.entities[&id(UNRELATED)],
        basis.entities[&id(UNRELATED)]
    );
    assert_eq!(outcome.project.entities.len(), basis.entities.len() + 2);
}

#[test]
fn conflicts_cover_binding_closure_slots_caps_and_embedded_dialog_rejection() {
    let (project, basis_head) = project();

    let duplicate = request(
        &project,
        &basis_head,
        Revision3QuestTranscriptIntentV1::Replace {
            bindings: vec![
                binding(&project, LINE_A, None),
                binding(&project, LINE_A, Some(1)),
            ],
        },
    );
    assert!(matches!(
        rejected(evaluate(&project, &basis_head, &duplicate)),
        Revision3QuestTranscriptEditConflictV1::DuplicateLine { .. }
    ));

    let inactive = request(
        &project,
        &basis_head,
        Revision3QuestTranscriptIntentV1::Replace {
            bindings: vec![binding(&project, LINE_A, Some(99))],
        },
    );
    assert!(matches!(
        rejected(evaluate(&project, &basis_head, &inactive)),
        Revision3QuestTranscriptEditConflictV1::InactiveObjectiveSlot { slot: 99, .. }
    ));

    let mut foreign = binding(&project, LINE_A, None);
    foreign.line.project_id = project_id(0xfe);
    let foreign = request(
        &project,
        &basis_head,
        Revision3QuestTranscriptIntentV1::Replace {
            bindings: vec![foreign],
        },
    );
    assert!(matches!(
        rejected(evaluate(&project, &basis_head, &foreign)),
        Revision3QuestTranscriptEditConflictV1::InvalidLineReference { .. }
    ));

    let too_many = request(
        &project,
        &basis_head,
        Revision3QuestTranscriptIntentV1::Replace {
            bindings: vec![
                binding(&project, LINE_A, None);
                MAX_REVISION3_QUEST_TRANSCRIPT_BINDINGS_V1 + 1
            ],
        },
    );
    assert!(matches!(
        rejected(evaluate(&project, &basis_head, &too_many)),
        Revision3QuestTranscriptEditConflictV1::TooManyBindings { .. }
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
        Revision3QuestTranscriptIntentV1::CreateAndInsert {
            index: 0,
            objective_slot: None,
            line: embedded_duplicate,
        },
    );
    assert!(matches!(
        rejected(evaluate(&project, &basis_head, &create)),
        Revision3QuestTranscriptEditConflictV1::DialogLineRejected { .. }
    ));
    assert_eq!(project.revision, 7);
    assert_eq!(project.entities.len(), 7);
}

#[test]
fn legacy_quests_reject_objective_slots_but_allow_unassigned_transcript() {
    let (mut project, basis_head) = project();
    let quest_id = id(QUEST);
    let module_id = id(MODULE);
    let Revision3EntityPayload::QuestDraft(quest) =
        &mut project.entities.get_mut(&quest_id).unwrap().payload
    else {
        panic!("fixture Quest kind")
    };
    quest.generator_version = 3;
    quest.input.transition_plan = None;
    let regenerated = regenerate_revision3_quest_module_v2(quest, collision_input(quest)).unwrap();
    let module_entity = project.entities.get_mut(&module_id).unwrap();
    let Revision3EntityPayload::ScriptModule(module) = &mut module_entity.payload else {
        panic!("fixture module kind")
    };
    *module = regenerated;
    let Revision3OriginRef::Generated {
        generator_version, ..
    } = &mut module_entity.origin
    else {
        panic!("fixture module origin")
    };
    *generator_version = 3;
    project.validate_closed_model().unwrap();

    let slotted = request(
        &project,
        &basis_head,
        Revision3QuestTranscriptIntentV1::Replace {
            bindings: vec![binding(&project, LINE_A, Some(1))],
        },
    );
    assert!(matches!(
        rejected(evaluate(&project, &basis_head, &slotted)),
        Revision3QuestTranscriptEditConflictV1::LegacyObjectiveSlot { slot: 1, .. }
    ));

    let unassigned = request(
        &project,
        &basis_head,
        Revision3QuestTranscriptIntentV1::Replace {
            bindings: vec![binding(&project, LINE_A, None)],
        },
    );
    assert_eq!(
        applied(evaluate(&project, &basis_head, &unassigned)).transcript_count,
        1
    );
}

// Keep this import live as an explicit assertion that transcript edits never rewrite module
// payloads, even when the transaction is composed with dialog creation.
fn _module_type_is_part_of_the_contract(_: &Revision3ScriptModule) {}
