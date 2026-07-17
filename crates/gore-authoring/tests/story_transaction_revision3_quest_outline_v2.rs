use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::{
    apply_revision3_quest_outline_edit_transaction_v2, regenerate_revision3_quest_module_v2,
    revision3_quest_transition_plan_basis_v1, revision3_quest_transition_plan_seal_v1, AssetMeta,
    AssetStoreIndex, ContentSeal, EntityId, FormatV2, GameGenerationAnchor, ProjectId, ProjectMeta,
    ProjectRevision3, QuestCollisionArtifactRef, QuestCollisionCatalogInput,
    QuestTransitionConditionAtomV1, QuestTransitionConditionGroupV1, QuestTransitionEdgeV1,
    QuestTransitionEffectKindV1, QuestTransitionEffectV1, QuestTransitionNodeV1,
    QuestTransitionPlanV1, QuestTransitionPredicateV1, QuestTransitionStateTestV1,
    Revision2DialogLine, Revision2LocalizationEntry, Revision3Entity, Revision3EntityKind,
    Revision3EntityPayload, Revision3OriginRef, Revision3QuestDraft, Revision3QuestDraftInput,
    Revision3QuestGiverInput, Revision3QuestOutlineEditBuildStatusV2,
    Revision3QuestOutlineEditConflictV2, Revision3QuestOutlineEditErrorV2,
    Revision3QuestOutlineEditEvaluationV2, Revision3QuestOutlineEditPublicationStatusV2,
    Revision3QuestOutlineEditRequestJsonErrorV2, Revision3QuestOutlineEditRequestV2,
    Revision3QuestOutlineEditRuntimeStatusV2, Revision3QuestOutlineObjectiveEditV2,
    Revision3QuestParentInput, Revision3QuestTranscriptBindingV1, Revision3ScriptModule,
    Revision3TypedRef, SchemaRevisionV3, Sha256Digest, WorkingHead, WorkingStoreFormat,
    MAX_REVISION3_QUEST_OUTLINE_EDIT_DISPLAY_NAME_BYTES_V2,
    MAX_REVISION3_QUEST_OUTLINE_EDIT_REQUEST_JSON_BYTES_V2, QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2,
    QUEST_COLLISION_CATALOG_LAYER_V2, REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION,
    REVISION3_QUEST_GENERATOR_ID, REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION,
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
        snapshot: seal(value, 4096),
    }
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

fn semantic_plan() -> QuestTransitionPlanV1 {
    let mut plan = QuestTransitionPlanV1::legacy_seed(3).unwrap();
    let root_start = plan
        .transitions
        .iter_mut()
        .find(|transition| {
            transition.node == QuestTransitionNodeV1::Root
                && transition.edge == QuestTransitionEdgeV1::Start
        })
        .unwrap();
    root_start.effects.push(QuestTransitionEffectV1 {
        target: QuestTransitionNodeV1::Objective { slot: 2 },
        effect: QuestTransitionEffectKindV1::Start,
    });
    let objective_one_availability = plan
        .transitions
        .iter_mut()
        .find(|transition| {
            transition.node == QuestTransitionNodeV1::Objective { slot: 1 }
                && transition.edge == QuestTransitionEdgeV1::Availability
        })
        .unwrap();
    objective_one_availability.external_allowed = false;
    objective_one_availability.predicate = Some(QuestTransitionPredicateV1 {
        any_of: vec![QuestTransitionConditionGroupV1 {
            all_of: vec![QuestTransitionConditionAtomV1 {
                node: QuestTransitionNodeV1::Root,
                test: QuestTransitionStateTestV1::Running,
                negated: false,
            }],
        }],
    });
    gore_authoring::validate_draft_quest_transition_plan_v1(&plan, 3).unwrap();
    plan
}

fn project_with_semantic_quest() -> (ProjectRevision3, WorkingHead) {
    let project_id = project_id(0x11);
    let quest_id = id(0x21);
    let module_id = id(0x22);
    let unrelated_id = id(0x30);
    let generation = target(0x31);
    let artifact = seal(0x41, 8192);
    let extra_asset = seal(0x44, 77);
    let plan = semantic_plan();
    let quest = Revision3QuestDraft {
        generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
        generator_version: REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION,
        input: Revision3QuestDraftInput {
            target: generation.clone(),
            quest_id,
            module_namespace: "GoreMods.Quests.AsghanTrial".to_owned(),
            technical_id: "GORE_ASGHAN_TRIAL".to_owned(),
            text_helper: "GoreAsghanTrialText".to_owned(),
            parent_quest: Revision3QuestParentInput {
                generation: generation.clone(),
                source_seal: seal(0x51, 4000),
                catalog_layer: "base-game.g1r.quests".to_owned(),
                canonical_selector: "CatalogQuest_SwampCamp_SCCHAPTER2".to_owned(),
                runtime_class: "UQuest_SwampCamp_SCCHAPTER2".to_owned(),
            },
            giver: Revision3QuestGiverInput {
                generation: generation.clone(),
                source_seal: seal(0x52, 5000),
                catalog_layer: "base-game.g1r.characters".to_owned(),
                canonical_selector: "CatalogCharacter_Asghan".to_owned(),
                runtime_unique_name: "OM_GRD_Asghan_263".to_owned(),
            },
            title: "Asghan's Trial".to_owned(),
            description: "Prove yourself without changing the technical Quest closure.".to_owned(),
            objective_title: "Enter the arena".to_owned(),
            additional_objective_titles: vec![
                "Defeat the guard".to_owned(),
                "Report to Asghan".to_owned(),
            ],
            transition_plan: Some(Box::new(plan)),
            collision_catalog: QuestCollisionArtifactRef {
                generation: generation.clone(),
                catalog_layer: QUEST_COLLISION_CATALOG_LAYER_V2.to_owned(),
                artifact: artifact.clone(),
                source_seal: seal(0x42, artifact.byte_len),
                basis_snapshot: seal(0x43, 4096),
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
    let project = ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id,
        revision: 7,
        meta: ProjectMeta {
            name: "Quest outline-v2 edit tests".to_owned(),
            version: "1.0.0".to_owned(),
            author: "tests".to_owned(),
        },
        target: generation,
        authoring_locales: BTreeSet::new(),
        entities: BTreeMap::from([
            (
                quest_id,
                Revision3Entity {
                    id: quest_id,
                    display_name: "Asghan Trial".to_owned(),
                    origin: Revision3OriginRef::New {
                        authored_runtime_id: "GORE_ASGHAN_TRIAL".to_owned(),
                    },
                    revision: 3,
                    payload: Revision3EntityPayload::QuestDraft(quest),
                },
            ),
            (
                module_id,
                Revision3Entity {
                    id: module_id,
                    display_name: "Preserved module label".to_owned(),
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
                unrelated_id,
                Revision3Entity {
                    id: unrelated_id,
                    display_name: "Unrelated localization".to_owned(),
                    origin: Revision3OriginRef::New {
                        authored_runtime_id: "GORE_UNRELATED_LOC".to_owned(),
                    },
                    revision: 9,
                    payload: Revision3EntityPayload::LocalizationEntry(
                        Revision2LocalizationEntry {
                            loc_id: "GORE_UNRELATED_LOC".to_owned(),
                            texts: BTreeMap::new(),
                        },
                    ),
                },
            ),
        ]),
        asset_store: AssetStoreIndex {
            assets: BTreeMap::from([
                (
                    artifact.sha256,
                    AssetMeta {
                        byte_len: artifact.byte_len,
                        media_type: QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2.to_owned(),
                    },
                ),
                (
                    extra_asset.sha256,
                    AssetMeta {
                        byte_len: extra_asset.byte_len,
                        media_type: "application/octet-stream".to_owned(),
                    },
                ),
            ]),
        },
    };
    project.validate_closed_model().unwrap();
    (project, head(0x61))
}

fn quest(project: &ProjectRevision3) -> &Revision3QuestDraft {
    let Revision3EntityPayload::QuestDraft(quest) = &project.entities[&id(0x21)].payload else {
        panic!("fixture Quest kind")
    };
    quest
}

fn module(project: &ProjectRevision3) -> &Revision3ScriptModule {
    let Revision3EntityPayload::ScriptModule(module) = &project.entities[&id(0x22)].payload else {
        panic!("fixture module kind")
    };
    module
}

fn current_objectives(project: &ProjectRevision3) -> Vec<Revision3QuestOutlineObjectiveEditV2> {
    let quest = quest(project);
    let plan = quest.input.transition_plan.as_deref().unwrap();
    let titles = std::iter::once(quest.input.objective_title.as_str()).chain(
        quest
            .input
            .additional_objective_titles
            .iter()
            .map(String::as_str),
    );
    plan.objective_order
        .iter()
        .copied()
        .zip(titles)
        .map(|(slot, title)| Revision3QuestOutlineObjectiveEditV2 {
            slot,
            title: title.to_owned(),
        })
        .collect()
}

fn request(
    project: &ProjectRevision3,
    basis_head: &WorkingHead,
) -> Revision3QuestOutlineEditRequestV2 {
    Revision3QuestOutlineEditRequestV2 {
        expected_head: basis_head.clone(),
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        expected_target: project.target.clone(),
        quest_id: id(0x21),
        expected_quest_revision: project.entities[&id(0x21)].revision,
        expected_script_module_id: id(0x22),
        expected_script_module_revision: project.entities[&id(0x22)].revision,
        expected_transition_plan_seal: revision3_quest_transition_plan_basis_v1(quest(project))
            .unwrap()
            .seal,
        display_name: project.entities[&id(0x21)].display_name.clone(),
        quest_title: quest(project).input.title.clone(),
        objectives: current_objectives(project),
    }
}

fn evaluate(
    project: &ProjectRevision3,
    basis_head: &WorkingHead,
    request: &Revision3QuestOutlineEditRequestV2,
) -> Result<Revision3QuestOutlineEditEvaluationV2, Revision3QuestOutlineEditErrorV2> {
    apply_revision3_quest_outline_edit_transaction_v2(
        basis_head,
        &project.to_canonical_json().unwrap(),
        &request.to_canonical_json().unwrap(),
    )
}

fn applied(
    project: &ProjectRevision3,
    basis_head: &WorkingHead,
    request: &Revision3QuestOutlineEditRequestV2,
) -> Box<gore_authoring::Revision3QuestOutlineEditOutcomeV2> {
    match evaluate(project, basis_head, request).unwrap() {
        Revision3QuestOutlineEditEvaluationV2::Applied(outcome) => outcome,
        Revision3QuestOutlineEditEvaluationV2::Rejected(rejection) => {
            panic!("unexpected rejection: {}", rejection.conflict)
        }
    }
}

fn rejected(
    project: &ProjectRevision3,
    basis_head: &WorkingHead,
    request: &Revision3QuestOutlineEditRequestV2,
) -> Revision3QuestOutlineEditConflictV2 {
    match evaluate(project, basis_head, request).unwrap() {
        Revision3QuestOutlineEditEvaluationV2::Rejected(rejection) => rejection.conflict,
        Revision3QuestOutlineEditEvaluationV2::Applied(_) => panic!("expected rejection"),
    }
}

fn attach_transcript(
    project: &mut ProjectRevision3,
    objective_slot: Option<u16>,
) -> Vec<Revision3QuestTranscriptBindingV1> {
    let localization_id = id(0x70);
    let line_id = id(0x71);
    project.entities.insert(
        localization_id,
        Revision3Entity {
            id: localization_id,
            display_name: "Outline-v2 preserved transcript text".to_owned(),
            origin: Revision3OriginRef::New {
                authored_runtime_id: "GORE_OUTLINE_V2_TRANSCRIPT_LOC_ENTITY".to_owned(),
            },
            revision: 2,
            payload: Revision3EntityPayload::LocalizationEntry(Revision2LocalizationEntry {
                loc_id: "GORE_OUTLINE_V2_TRANSCRIPT_TEXT".to_owned(),
                texts: BTreeMap::new(),
            }),
        },
    );
    project.entities.insert(
        line_id,
        Revision3Entity {
            id: line_id,
            display_name: "Outline-v2 preserved transcript line".to_owned(),
            origin: Revision3OriginRef::New {
                authored_runtime_id: "GORE_OUTLINE_V2_TRANSCRIPT_LINE".to_owned(),
            },
            revision: 4,
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
    let transcript = vec![Revision3QuestTranscriptBindingV1 {
        line: Revision3TypedRef::new(project.project_id, line_id, Revision3EntityKind::DialogLine),
        objective_slot,
    }];
    let Revision3EntityPayload::QuestDraft(quest) =
        &mut project.entities.get_mut(&id(0x21)).unwrap().payload
    else {
        panic!("fixture Quest kind")
    };
    quest.transcript = transcript.clone();
    project.validate_closed_model().unwrap();
    transcript
}

#[test]
fn edit_reorders_stable_slots_and_changes_only_the_bounded_outline_and_three_revisions() {
    let (mut project, basis_head) = project_with_semantic_quest();
    let transcript = attach_transcript(&mut project, Some(2));
    let before = project.clone();
    let before_quest = quest(&before).clone();
    let before_plan = before_quest
        .input
        .transition_plan
        .as_deref()
        .unwrap()
        .clone();
    let before_module_entity = before.entities[&id(0x22)].clone();
    let mut request = request(&project, &basis_head);
    request.display_name = "Asghan's Arena Trial".to_owned();
    request.quest_title = "The Arena Trial".to_owned();
    request.objectives = vec![
        Revision3QuestOutlineObjectiveEditV2 {
            slot: 3,
            title: "Report your victory".to_owned(),
        },
        Revision3QuestOutlineObjectiveEditV2 {
            slot: 1,
            title: "Enter Asghan's arena".to_owned(),
        },
        Revision3QuestOutlineObjectiveEditV2 {
            slot: 2,
            title: "Defeat the arena guard".to_owned(),
        },
    ];

    let outcome = applied(&project, &basis_head, &request);
    assert_eq!(outcome.basis_head, basis_head);
    assert_eq!(outcome.quest_id, id(0x21));
    assert_eq!(outcome.script_module_id, id(0x22));
    assert_eq!(outcome.quest_revision, 4);
    assert_eq!(outcome.script_module_revision, 6);
    assert_eq!(quest(&outcome.project).transcript, transcript);
    assert_eq!(
        outcome.build_status,
        Revision3QuestOutlineEditBuildStatusV2::Blocked
    );
    assert_eq!(
        outcome.runtime_status,
        Revision3QuestOutlineEditRuntimeStatusV2::RuntimeUnqualified
    );
    assert_eq!(
        outcome.publication_status,
        Revision3QuestOutlineEditPublicationStatusV2::NotSupported
    );

    assert_eq!(outcome.project.project_id, before.project_id);
    assert_eq!(outcome.project.revision, before.revision + 1);
    assert_eq!(outcome.project.meta, before.meta);
    assert_eq!(outcome.project.target, before.target);
    assert_eq!(outcome.project.authoring_locales, before.authoring_locales);
    assert_eq!(outcome.project.asset_store, before.asset_store);
    assert_eq!(outcome.project.entities.len(), before.entities.len());
    assert_eq!(
        outcome.project.entities[&id(0x30)],
        before.entities[&id(0x30)]
    );

    let mut expected_quest = before_quest;
    expected_quest.input.title = request.quest_title.clone();
    expected_quest.input.objective_title = request.objectives[0].title.clone();
    expected_quest.input.additional_objective_titles = request.objectives[1..]
        .iter()
        .map(|objective| objective.title.clone())
        .collect();
    let mut expected_plan = before_plan.clone();
    expected_plan.objective_order = vec![3, 1, 2];
    expected_quest.input.transition_plan = Some(Box::new(expected_plan.clone()));

    let after_quest_entity = &outcome.project.entities[&id(0x21)];
    assert_eq!(after_quest_entity.id, before.entities[&id(0x21)].id);
    assert_eq!(after_quest_entity.display_name, request.display_name);
    assert_eq!(after_quest_entity.origin, before.entities[&id(0x21)].origin);
    assert_eq!(
        after_quest_entity.revision,
        before.entities[&id(0x21)].revision + 1
    );
    assert_eq!(quest(&outcome.project), &expected_quest);
    assert_eq!(expected_plan.objective_slots, before_plan.objective_slots);
    assert_eq!(
        expected_plan.next_slot_ordinal,
        before_plan.next_slot_ordinal
    );
    assert_eq!(expected_plan.transitions, before_plan.transitions);

    let after_module_entity = &outcome.project.entities[&id(0x22)];
    assert_eq!(after_module_entity.id, before_module_entity.id);
    assert_eq!(
        after_module_entity.display_name,
        before_module_entity.display_name
    );
    assert_eq!(after_module_entity.origin, before_module_entity.origin);
    assert_eq!(
        after_module_entity.revision,
        before_module_entity.revision + 1
    );
    let after_module = module(&outcome.project);
    let before_module = module(&before);
    assert_eq!(after_module.owner, before_module.owner);
    assert_eq!(after_module.generator_id, before_module.generator_id);
    assert_eq!(
        after_module.generator_version,
        before_module.generator_version
    );
    assert_eq!(
        after_module.module_namespace,
        before_module.module_namespace
    );
    assert_eq!(
        after_module.module_relative_path,
        before_module.module_relative_path
    );
    assert_eq!(after_module.status, before_module.status);
    assert_ne!(after_module.source, before_module.source);
    assert_eq!(
        after_module,
        &regenerate_revision3_quest_module_v2(
            quest(&outcome.project),
            collision_input(quest(&outcome.project)),
        )
        .unwrap()
    );
    assert_eq!(
        outcome.transition_plan_seal,
        revision3_quest_transition_plan_seal_v1(&expected_plan).unwrap()
    );

    let reopened = ProjectRevision3::from_json(&outcome.canonical_project_json).unwrap();
    assert_eq!(reopened, outcome.project);
    assert_eq!(
        outcome.canonical_project_json,
        outcome.project.to_canonical_json().unwrap()
    );
}

#[test]
fn reorder_with_unchanged_slot_titles_keeps_each_title_attached_to_its_stable_slot() {
    let (project, basis_head) = project_with_semantic_quest();
    let mut request = request(&project, &basis_head);
    request.objectives = vec![
        request.objectives[2].clone(),
        request.objectives[0].clone(),
        request.objectives[1].clone(),
    ];
    let outcome = applied(&project, &basis_head, &request);
    assert_eq!(current_objectives(&outcome.project), request.objectives);
    assert_eq!(
        quest(&outcome.project)
            .input
            .transition_plan
            .as_deref()
            .unwrap()
            .objective_order,
        vec![3, 1, 2]
    );
    assert_eq!(
        quest(&outcome.project).input.objective_title,
        "Report to Asghan"
    );
    assert_eq!(
        quest(&outcome.project).input.additional_objective_titles,
        vec!["Enter the arena", "Defeat the guard"]
    );
}

#[test]
fn exact_same_outline_is_a_no_op_but_each_editable_surface_can_change_independently() {
    let (project, basis_head) = project_with_semantic_quest();
    let unchanged = request(&project, &basis_head);
    assert_eq!(
        rejected(&project, &basis_head, &unchanged),
        Revision3QuestOutlineEditConflictV2::NoChanges
    );

    let mut name_only = unchanged.clone();
    name_only.display_name = "Renamed in library".to_owned();
    assert_eq!(
        applied(&project, &basis_head, &name_only).project.entities[&id(0x21)].display_name,
        "Renamed in library"
    );

    let mut quest_title_only = unchanged.clone();
    quest_title_only.quest_title = "A Different Trial".to_owned();
    assert_eq!(
        quest(&applied(&project, &basis_head, &quest_title_only).project)
            .input
            .title,
        "A Different Trial"
    );

    let mut objective_title_only = unchanged;
    objective_title_only.objectives[1].title = "Defeat Kirgo".to_owned();
    assert_eq!(
        current_objectives(&applied(&project, &basis_head, &objective_title_only).project)[1].title,
        "Defeat Kirgo"
    );
}

#[test]
fn exact_head_project_quest_module_and_plan_cas_are_all_enforced() {
    let (project, basis_head) = project_with_semantic_quest();
    let mut base = request(&project, &basis_head);
    base.display_name = "Changed".to_owned();

    let mut candidate = base.clone();
    candidate.expected_head = head(0x70);
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV2::CurrentHeadMismatch
    );
    candidate = base.clone();
    candidate.expected_project_id = project_id(0x70);
    assert!(matches!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV2::ProjectIdentityMismatch { .. }
    ));
    candidate = base.clone();
    candidate.expected_revision += 1;
    assert!(matches!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV2::ProjectRevisionConflict { .. }
    ));
    candidate = base.clone();
    candidate.expected_target = target(0x70);
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV2::ProjectTargetMismatch
    );
    candidate = base.clone();
    candidate.expected_quest_revision += 1;
    assert!(matches!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV2::QuestRevisionConflict { .. }
    ));
    candidate = base.clone();
    candidate.expected_script_module_id = id(0x70);
    assert!(matches!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV2::ScriptModuleIdentityConflict { .. }
    ));
    candidate = base.clone();
    candidate.expected_script_module_revision += 1;
    assert!(matches!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV2::ScriptModuleRevisionConflict { .. }
    ));
    candidate = base;
    candidate.expected_transition_plan_seal = seal(0x70, 1);
    assert!(matches!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV2::TransitionPlanSealConflict { .. }
    ));
}

#[test]
fn added_removed_duplicate_zero_and_foreign_slots_fail_closed() {
    let (project, basis_head) = project_with_semantic_quest();
    let base = request(&project, &basis_head);

    let mut candidate = base.clone();
    candidate
        .objectives
        .push(Revision3QuestOutlineObjectiveEditV2 {
            slot: 4,
            title: "Added".to_owned(),
        });
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV2::ForeignObjectiveSlot { slot: 4 }
    );

    candidate = base.clone();
    candidate.objectives.pop();
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV2::MissingObjectiveSlot { slot: 3 }
    );

    candidate = base.clone();
    candidate.objectives[1].slot = 1;
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV2::DuplicateObjectiveSlot { slot: 1 }
    );

    candidate = base.clone();
    candidate.objectives[2].slot = 99;
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV2::ForeignObjectiveSlot { slot: 99 }
    );

    candidate = base;
    candidate.objectives[0].slot = 0;
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV2::ForeignObjectiveSlot { slot: 0 }
    );
}

#[test]
fn input_and_text_bounds_are_closed_before_candidate_mutation() {
    let (project, basis_head) = project_with_semantic_quest();
    let base = request(&project, &basis_head);

    for display_name in [
        "".to_owned(),
        " leading".to_owned(),
        "trailing ".to_owned(),
        "line\nbreak".to_owned(),
        "x".repeat(MAX_REVISION3_QUEST_OUTLINE_EDIT_DISPLAY_NAME_BYTES_V2 + 1),
    ] {
        let mut candidate = base.clone();
        candidate.display_name = display_name;
        assert_eq!(
            rejected(&project, &basis_head, &candidate),
            Revision3QuestOutlineEditConflictV2::InvalidDisplayName
        );
    }

    let mut candidate = base.clone();
    candidate.objectives.clear();
    assert!(matches!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV2::InvalidObjectiveCount { actual: 0, .. }
    ));
    candidate = base.clone();
    candidate.objectives = (1..=9)
        .map(|slot| Revision3QuestOutlineObjectiveEditV2 {
            slot,
            title: format!("Objective {slot}"),
        })
        .collect();
    assert!(matches!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV2::InvalidObjectiveCount { actual: 9, .. }
    ));

    candidate = base.clone();
    candidate.objectives[0].title = " leading".to_owned();
    assert!(matches!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV2::InvalidObjectiveTitles { .. }
    ));

    candidate = base;
    candidate.quest_title = "Invalid\nQuest title".to_owned();
    assert!(matches!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV2::InvalidOutlineText { .. }
    ));
}

#[test]
fn canonical_request_parser_rejects_malformed_unknown_duplicate_noncanonical_and_oversized_json() {
    let (project, basis_head) = project_with_semantic_quest();
    let request = request(&project, &basis_head);
    let json = request.to_canonical_json().unwrap();
    assert_eq!(
        Revision3QuestOutlineEditRequestV2::from_json(&json).unwrap(),
        request
    );

    assert!(matches!(
        Revision3QuestOutlineEditRequestV2::from_json("{"),
        Err(Revision3QuestOutlineEditRequestJsonErrorV2::InvalidJson(_))
    ));
    let unknown = json.replacen('{', "{\"unknown\":true,", 1);
    assert!(matches!(
        Revision3QuestOutlineEditRequestV2::from_json(&unknown),
        Err(Revision3QuestOutlineEditRequestJsonErrorV2::InvalidJson(_))
    ));
    let nested_unknown = json.replacen("\"slot\":1", "\"slot\":1,\"unknown\":true", 1);
    assert!(matches!(
        Revision3QuestOutlineEditRequestV2::from_json(&nested_unknown),
        Err(Revision3QuestOutlineEditRequestJsonErrorV2::InvalidJson(_))
    ));
    let duplicate = json.replacen(
        '{',
        &format!("{{\"expected_revision\":{},", request.expected_revision),
        1,
    );
    assert!(matches!(
        Revision3QuestOutlineEditRequestV2::from_json(&duplicate),
        Err(Revision3QuestOutlineEditRequestJsonErrorV2::InvalidJson(_))
    ));
    let nested_duplicate = json.replacen("\"slot\":1", "\"slot\":1,\"slot\":1", 1);
    assert!(matches!(
        Revision3QuestOutlineEditRequestV2::from_json(&nested_duplicate),
        Err(Revision3QuestOutlineEditRequestJsonErrorV2::InvalidJson(_))
    ));
    assert!(matches!(
        Revision3QuestOutlineEditRequestV2::from_json(&format!(" {json}")),
        Err(Revision3QuestOutlineEditRequestJsonErrorV2::NonCanonicalJson)
    ));
    assert!(matches!(
        Revision3QuestOutlineEditRequestV2::from_json(
            &"x".repeat(MAX_REVISION3_QUEST_OUTLINE_EDIT_REQUEST_JSON_BYTES_V2 + 1)
        ),
        Err(Revision3QuestOutlineEditRequestJsonErrorV2::InputTooLarge { .. })
    ));

    let mut oversized = request;
    oversized.display_name = "x".repeat(MAX_REVISION3_QUEST_OUTLINE_EDIT_REQUEST_JSON_BYTES_V2 + 1);
    assert!(matches!(
        oversized.to_canonical_json(),
        Err(Revision3QuestOutlineEditRequestJsonErrorV2::InputTooLarge { .. })
    ));
}

#[test]
fn legacy_quest_owned_module_drift_and_revision_overflow_are_rejected() {
    let (project, basis_head) = project_with_semantic_quest();
    let mut edit = request(&project, &basis_head);
    edit.display_name = "Changed".to_owned();

    let mut legacy = project.clone();
    let Revision3EntityPayload::QuestDraft(legacy_quest) =
        &mut legacy.entities.get_mut(&id(0x21)).unwrap().payload
    else {
        panic!("Quest")
    };
    legacy_quest.generator_version = REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION;
    legacy_quest.input.transition_plan = None;
    let legacy_module =
        regenerate_revision3_quest_module_v2(legacy_quest, collision_input(legacy_quest)).unwrap();
    let module_entity = legacy.entities.get_mut(&id(0x22)).unwrap();
    let Revision3OriginRef::Generated {
        generator_version, ..
    } = &mut module_entity.origin
    else {
        panic!("generated")
    };
    *generator_version = REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION;
    module_entity.payload = Revision3EntityPayload::ScriptModule(legacy_module);
    legacy.validate_closed_model().unwrap();
    assert!(matches!(
        rejected(&legacy, &basis_head, &edit),
        Revision3QuestOutlineEditConflictV2::SemanticQuestRequired { .. }
    ));

    let mut drifted = project.clone();
    let Revision3EntityPayload::ScriptModule(drifted_module) =
        &mut drifted.entities.get_mut(&id(0x22)).unwrap().payload
    else {
        panic!("module")
    };
    drifted_module.source.push_str("\n// drift");
    drifted_module.source_sha256 =
        Sha256Digest::from_bytes(Sha256::digest(drifted_module.source.as_bytes()).into());
    drifted.validate_closed_model().unwrap();
    assert!(matches!(
        rejected(&drifted, &basis_head, &edit),
        Revision3QuestOutlineEditConflictV2::OwnedModuleDrift { .. }
    ));

    let mut overflow = project.clone();
    overflow.revision = u64::MAX;
    let mut overflow_edit = edit.clone();
    overflow_edit.expected_revision = u64::MAX;
    assert_eq!(
        rejected(&overflow, &basis_head, &overflow_edit),
        Revision3QuestOutlineEditConflictV2::ProjectRevisionOverflow
    );

    overflow = project.clone();
    overflow.entities.get_mut(&id(0x21)).unwrap().revision = u64::MAX;
    overflow_edit = edit.clone();
    overflow_edit.expected_quest_revision = u64::MAX;
    assert!(matches!(
        rejected(&overflow, &basis_head, &overflow_edit),
        Revision3QuestOutlineEditConflictV2::QuestRevisionOverflow { .. }
    ));

    overflow = project;
    overflow.entities.get_mut(&id(0x22)).unwrap().revision = u64::MAX;
    overflow_edit = edit;
    overflow_edit.expected_script_module_revision = u64::MAX;
    assert!(matches!(
        rejected(&overflow, &basis_head, &overflow_edit),
        Revision3QuestOutlineEditConflictV2::ScriptModuleRevisionOverflow { .. }
    ));
}
