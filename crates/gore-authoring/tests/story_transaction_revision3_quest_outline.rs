use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::{
    apply_revision3_quest_outline_edit_transaction_v1, regenerate_revision3_quest_module_v2,
    AssetMeta, AssetStoreIndex, ContentSeal, DraftQuestField, DraftQuestSkeletonError, EntityId,
    FormatV2, GameGenerationAnchor, ProjectId, ProjectMeta, ProjectRevision3,
    QuestCollisionArtifactRef, QuestCollisionCatalogInput, QuestTransitionPlanV1, Revision3Entity,
    Revision3EntityKind, Revision3EntityPayload, Revision3OriginRef, Revision3QuestDraft,
    Revision3QuestDraftInput, Revision3QuestGiverInput, Revision3QuestOutlineEditBuildStatusV1,
    Revision3QuestOutlineEditConflictV1, Revision3QuestOutlineEditErrorV1,
    Revision3QuestOutlineEditEvaluationV1, Revision3QuestOutlineEditRequestJsonErrorV1,
    Revision3QuestOutlineEditRequestV1, Revision3QuestOutlineEditRuntimeStatusV1,
    Revision3QuestParentInput, Revision3ScriptModule, Revision3TypedRef, SchemaRevisionV3,
    Sha256Digest, WorkingHead, WorkingStoreFormat, MAX_DRAFT_QUEST_OBJECTIVES,
    MAX_DRAFT_QUEST_OBJECTIVE_TITLE_BYTES, MAX_DRAFT_QUEST_TITLE_BYTES, MAX_PROJECT_JSON_BYTES,
    MAX_REVISION3_QUEST_OUTLINE_EDIT_DISPLAY_NAME_BYTES_V1,
    MAX_REVISION3_QUEST_OUTLINE_EDIT_REQUEST_JSON_BYTES_V1, QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2,
    QUEST_COLLISION_CATALOG_LAYER_V2, REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION,
    REVISION3_QUEST_GENERATOR_ID, REVISION3_QUEST_GENERATOR_VERSION,
    REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION,
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

fn project_with_quest(objective_titles: &[&str]) -> (ProjectRevision3, WorkingHead) {
    assert!(!objective_titles.is_empty());
    assert!(objective_titles.len() <= MAX_DRAFT_QUEST_OBJECTIVES);
    let project_id = project_id(0x11);
    let quest_id = id(0x21);
    let module_id = id(0x22);
    let generation = target(0x31);
    let artifact = seal(0x41, 8192);
    let source_seal = seal(0x42, artifact.byte_len);
    let artifact_ref = QuestCollisionArtifactRef {
        generation: generation.clone(),
        catalog_layer: QUEST_COLLISION_CATALOG_LAYER_V2.to_owned(),
        artifact: artifact.clone(),
        source_seal: source_seal.clone(),
        basis_snapshot: seal(0x43, 4096),
    };
    let generator_version = if objective_titles.len() == 1 {
        REVISION3_QUEST_GENERATOR_VERSION
    } else {
        REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION
    };
    let quest = Revision3QuestDraft {
        generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
        generator_version,
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
            objective_title: objective_titles[0].to_owned(),
            additional_objective_titles: objective_titles[1..]
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            transition_plan: None,
            collision_catalog: artifact_ref,
        },
        script_module: Revision3TypedRef::new(
            project_id,
            module_id,
            Revision3EntityKind::ScriptModule,
        ),
    };
    let module = regenerate_revision3_quest_module_v2(&quest, collision_input(&quest)).unwrap();
    let owner = Revision3TypedRef::new(project_id, quest_id, Revision3EntityKind::QuestDraft);
    let entities = BTreeMap::from([
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
                    generator_version,
                    owner,
                },
                revision: 5,
                payload: Revision3EntityPayload::ScriptModule(module),
            },
        ),
    ]);
    let project = ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id,
        revision: 7,
        meta: ProjectMeta {
            name: "Quest outline edit tests".to_owned(),
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
    (project, head(0x61))
}

fn request(
    project: &ProjectRevision3,
    basis_head: &WorkingHead,
) -> Revision3QuestOutlineEditRequestV1 {
    let quest_id = id(0x21);
    let quest_entity = &project.entities[&quest_id];
    let Revision3EntityPayload::QuestDraft(quest) = &quest_entity.payload else {
        panic!("fixture Quest kind")
    };
    let mut objectives = vec!["Earn Asghan's trust".to_owned()];
    objectives.extend(
        quest
            .input
            .additional_objective_titles
            .iter()
            .enumerate()
            .map(|(index, _)| format!("Complete trial stage {}", index + 2)),
    );
    Revision3QuestOutlineEditRequestV1 {
        expected_head: basis_head.clone(),
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        expected_target: project.target.clone(),
        quest_id,
        expected_quest_revision: quest_entity.revision,
        display_name: "Asghan's Revised Trial".to_owned(),
        title: "The Revised Trial".to_owned(),
        objective_titles: objectives,
    }
}

fn evaluate(
    project: &ProjectRevision3,
    basis_head: &WorkingHead,
    request: &Revision3QuestOutlineEditRequestV1,
) -> Result<Revision3QuestOutlineEditEvaluationV1, Revision3QuestOutlineEditErrorV1> {
    apply_revision3_quest_outline_edit_transaction_v1(
        basis_head,
        &project.to_canonical_json().unwrap(),
        &request.to_canonical_json().unwrap(),
    )
}

fn rejected(
    project: &ProjectRevision3,
    basis_head: &WorkingHead,
    request: &Revision3QuestOutlineEditRequestV1,
) -> Revision3QuestOutlineEditConflictV1 {
    match evaluate(project, basis_head, request).unwrap() {
        Revision3QuestOutlineEditEvaluationV1::Rejected(rejection) => rejection.conflict,
        Revision3QuestOutlineEditEvaluationV1::Applied(_) => panic!("expected rejection"),
    }
}

#[test]
fn exact_edit_changes_only_outline_module_and_three_revisions() {
    let (project, basis_head) = project_with_quest(&["Win the trial"]);
    let request = request(&project, &basis_head);
    let before = project.clone();
    let outcome = match evaluate(&project, &basis_head, &request).unwrap() {
        Revision3QuestOutlineEditEvaluationV1::Applied(outcome) => outcome,
        Revision3QuestOutlineEditEvaluationV1::Rejected(rejection) => {
            panic!("unexpected rejection: {}", rejection.conflict)
        }
    };

    assert_eq!(outcome.basis_head, basis_head);
    assert_eq!(outcome.quest_id, id(0x21));
    assert_eq!(outcome.script_module_id, id(0x22));
    assert_eq!(outcome.quest_revision, 4);
    assert_eq!(outcome.script_module_revision, 6);
    assert_eq!(
        outcome.build_status,
        Revision3QuestOutlineEditBuildStatusV1::Blocked
    );
    assert_eq!(
        outcome.runtime_status,
        Revision3QuestOutlineEditRuntimeStatusV1::RuntimeUnqualified
    );
    assert_eq!(outcome.project.revision, before.revision + 1);
    assert_eq!(outcome.project.meta, before.meta);
    assert_eq!(outcome.project.target, before.target);
    assert_eq!(outcome.project.authoring_locales, before.authoring_locales);
    assert_eq!(outcome.project.asset_store, before.asset_store);
    assert_eq!(outcome.project.entities.len(), before.entities.len());

    let before_quest_entity = &before.entities[&id(0x21)];
    let after_quest_entity = &outcome.project.entities[&id(0x21)];
    assert_eq!(after_quest_entity.id, before_quest_entity.id);
    assert_eq!(after_quest_entity.origin, before_quest_entity.origin);
    assert_eq!(
        after_quest_entity.revision,
        before_quest_entity.revision + 1
    );
    assert_eq!(after_quest_entity.display_name, request.display_name);
    let Revision3EntityPayload::QuestDraft(before_quest) = &before_quest_entity.payload else {
        panic!("fixture Quest kind")
    };
    let Revision3EntityPayload::QuestDraft(after_quest) = &after_quest_entity.payload else {
        panic!("edited Quest kind")
    };
    assert_eq!(after_quest.generator_id, before_quest.generator_id);
    assert_eq!(
        after_quest.generator_version,
        before_quest.generator_version
    );
    assert_eq!(after_quest.script_module, before_quest.script_module);
    assert_eq!(after_quest.input.target, before_quest.input.target);
    assert_eq!(after_quest.input.quest_id, before_quest.input.quest_id);
    assert_eq!(
        after_quest.input.module_namespace,
        before_quest.input.module_namespace
    );
    assert_eq!(
        after_quest.input.technical_id,
        before_quest.input.technical_id
    );
    assert_eq!(
        after_quest.input.text_helper,
        before_quest.input.text_helper
    );
    assert_eq!(
        after_quest.input.parent_quest,
        before_quest.input.parent_quest
    );
    assert_eq!(after_quest.input.giver, before_quest.input.giver);
    assert_eq!(
        after_quest.input.description,
        before_quest.input.description
    );
    assert_eq!(
        after_quest.input.collision_catalog,
        before_quest.input.collision_catalog
    );
    assert_eq!(after_quest.input.title, request.title);
    assert_eq!(
        after_quest.input.objective_title,
        request.objective_titles[0]
    );
    assert!(after_quest.input.additional_objective_titles.is_empty());

    let before_module_entity = &before.entities[&id(0x22)];
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
    let Revision3EntityPayload::ScriptModule(before_module) = &before_module_entity.payload else {
        panic!("fixture module kind")
    };
    let Revision3EntityPayload::ScriptModule(after_module) = &after_module_entity.payload else {
        panic!("edited module kind")
    };
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
    assert_ne!(
        after_module.input_fingerprint,
        before_module.input_fingerprint
    );
    assert_eq!(
        after_module.source_sha256,
        Sha256Digest::from_bytes(Sha256::digest(after_module.source.as_bytes()).into())
    );
    assert_eq!(
        after_module,
        &regenerate_revision3_quest_module_v2(after_quest, collision_input(after_quest)).unwrap()
    );

    let reopened = ProjectRevision3::from_json(&outcome.canonical_project_json).unwrap();
    assert_eq!(reopened, outcome.project);
    assert_eq!(
        outcome.canonical_project_json,
        outcome.project.to_canonical_json().unwrap()
    );
}

#[test]
fn eight_objective_edit_retains_order_count_and_generator_version() {
    let old_titles = [
        "Stage one",
        "Stage two",
        "Stage three",
        "Stage four",
        "Stage five",
        "Stage six",
        "Stage seven",
        "Stage eight",
    ];
    let (project, basis_head) = project_with_quest(&old_titles);
    let request = request(&project, &basis_head);
    let outcome = match evaluate(&project, &basis_head, &request).unwrap() {
        Revision3QuestOutlineEditEvaluationV1::Applied(outcome) => outcome,
        Revision3QuestOutlineEditEvaluationV1::Rejected(rejection) => {
            panic!("unexpected rejection: {}", rejection.conflict)
        }
    };
    let Revision3EntityPayload::QuestDraft(quest) = &outcome.project.entities[&id(0x21)].payload
    else {
        panic!("edited Quest kind")
    };
    assert_eq!(
        quest.generator_version,
        REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION
    );
    let actual: Vec<&str> = std::iter::once(quest.input.objective_title.as_str())
        .chain(
            quest
                .input
                .additional_objective_titles
                .iter()
                .map(String::as_str),
        )
        .collect();
    let expected: Vec<&str> = request
        .objective_titles
        .iter()
        .map(String::as_str)
        .collect();
    assert_eq!(actual, expected);
    let Revision3EntityPayload::ScriptModule(module) = &outcome.project.entities[&id(0x22)].payload
    else {
        panic!("edited module kind")
    };
    let positions: Vec<usize> = request
        .objective_titles
        .iter()
        .map(|title| module.source.find(title).expect("title emitted in source"))
        .collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn request_json_is_bounded_duplicate_free_and_exactly_canonical() {
    let (project, basis_head) = project_with_quest(&["Win the trial"]);
    let request = request(&project, &basis_head);
    let json = request.to_canonical_json().unwrap();
    assert_eq!(
        Revision3QuestOutlineEditRequestV1::from_json(&json).unwrap(),
        request
    );

    let noncanonical = format!(" {json}");
    assert!(matches!(
        Revision3QuestOutlineEditRequestV1::from_json(&noncanonical),
        Err(Revision3QuestOutlineEditRequestJsonErrorV1::NonCanonicalJson)
    ));
    let duplicate = json.replacen(
        "\"expected_revision\":7",
        "\"expected_revision\":7,\"expected_revision\":7",
        1,
    );
    assert!(matches!(
        Revision3QuestOutlineEditRequestV1::from_json(&duplicate),
        Err(Revision3QuestOutlineEditRequestJsonErrorV1::InvalidJson(_))
    ));
    let unknown = json.replacen('{', "{\"unknown\":false,", 1);
    assert!(matches!(
        Revision3QuestOutlineEditRequestV1::from_json(&unknown),
        Err(Revision3QuestOutlineEditRequestJsonErrorV1::InvalidJson(_))
    ));

    let mut oversized = request;
    oversized.display_name = "x".repeat(MAX_REVISION3_QUEST_OUTLINE_EDIT_REQUEST_JSON_BYTES_V1);
    assert!(matches!(
        oversized.to_canonical_json(),
        Err(Revision3QuestOutlineEditRequestJsonErrorV1::InputTooLarge { .. })
    ));
    let oversized_json = format!(
        "{{\"padding\":\"{}\"}}",
        "x".repeat(MAX_REVISION3_QUEST_OUTLINE_EDIT_REQUEST_JSON_BYTES_V1)
    );
    assert!(matches!(
        Revision3QuestOutlineEditRequestV1::from_json(&oversized_json),
        Err(Revision3QuestOutlineEditRequestJsonErrorV1::InputTooLarge { .. })
    ));
}

#[test]
fn exact_cas_and_entity_kind_conflicts_are_rejected_without_a_candidate() {
    let (project, basis_head) = project_with_quest(&["Win the trial"]);
    let base = request(&project, &basis_head);

    let mut candidate = base.clone();
    candidate.expected_head = head(0x62);
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV1::CurrentHeadMismatch
    );
    candidate = base.clone();
    candidate.expected_project_id = project_id(0x12);
    assert!(matches!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV1::ProjectIdentityMismatch { .. }
    ));
    candidate = base.clone();
    candidate.expected_revision -= 1;
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV1::ProjectRevisionConflict {
            expected: 6,
            actual: 7,
        }
    );
    candidate = base.clone();
    candidate.expected_target = target(0x32);
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV1::ProjectTargetMismatch
    );
    candidate = base.clone();
    candidate.expected_quest_revision -= 1;
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV1::QuestRevisionConflict {
            expected: 2,
            actual: 3,
        }
    );
    candidate = base.clone();
    candidate.quest_id = EntityId::from_bytes([0; 16]);
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV1::ZeroQuestId
    );
    candidate = base;
    candidate.quest_id = id(0x22);
    candidate.expected_quest_revision = 5;
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV1::InvalidQuestEntity { quest: id(0x22) }
    );
}

#[test]
fn no_op_and_objective_shape_changes_are_rejected() {
    let (project, basis_head) = project_with_quest(&["Win the trial"]);
    let mut candidate = request(&project, &basis_head);
    let Revision3EntityPayload::QuestDraft(quest) = &project.entities[&id(0x21)].payload else {
        panic!("fixture Quest kind")
    };
    candidate.display_name = project.entities[&id(0x21)].display_name.clone();
    candidate.title = quest.input.title.clone();
    candidate.objective_titles = vec![quest.input.objective_title.clone()];
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV1::NoChanges
    );
    assert_eq!(project.revision, 7);
    assert_eq!(project.entities[&id(0x21)].revision, 3);
    assert_eq!(project.entities[&id(0x22)].revision, 5);

    candidate = request(&project, &basis_head);
    candidate.objective_titles.clear();
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV1::InvalidObjectiveCount {
            actual: 0,
            max: MAX_DRAFT_QUEST_OBJECTIVES,
        }
    );
    candidate = request(&project, &basis_head);
    candidate.objective_titles = (0..=MAX_DRAFT_QUEST_OBJECTIVES)
        .map(|index| format!("Objective {index}"))
        .collect();
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV1::InvalidObjectiveCount {
            actual: MAX_DRAFT_QUEST_OBJECTIVES + 1,
            max: MAX_DRAFT_QUEST_OBJECTIVES,
        }
    );
    candidate = request(&project, &basis_head);
    candidate
        .objective_titles
        .push("Unexpected second objective".to_owned());
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV1::ObjectiveCountChange {
            expected: 1,
            actual: 2,
        }
    );
}

#[test]
fn semantic_quest_requires_stable_slot_aware_outline_v2() {
    let (mut project, basis_head) = project_with_quest(&["Stage one", "Stage two", "Stage three"]);
    let quest_id = id(0x21);
    let module_id = id(0x22);
    let mut quest = match &project.entities[&quest_id].payload {
        Revision3EntityPayload::QuestDraft(quest) => quest.clone(),
        _ => panic!("fixture Quest kind"),
    };
    quest.generator_version = REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION;
    quest.input.transition_plan = Some(Box::new(QuestTransitionPlanV1::legacy_seed(3).unwrap()));
    let module = regenerate_revision3_quest_module_v2(&quest, collision_input(&quest)).unwrap();
    project.entities.get_mut(&quest_id).unwrap().payload =
        Revision3EntityPayload::QuestDraft(quest);
    let module_entity = project.entities.get_mut(&module_id).unwrap();
    module_entity.payload = Revision3EntityPayload::ScriptModule(module);
    let Revision3OriginRef::Generated {
        generator_version, ..
    } = &mut module_entity.origin
    else {
        panic!("fixture module origin")
    };
    *generator_version = REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION;
    project.validate_closed_model().unwrap();

    let candidate = request(&project, &basis_head);
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV1::SemanticQuestRequiresOutlineV2
    );
    assert_eq!(project.revision, 7);
    assert_eq!(project.entities[&quest_id].revision, 3);
    assert_eq!(project.entities[&module_id].revision, 5);
}

#[test]
fn candidate_project_capacity_is_distinct_from_basis_corruption() {
    let (mut project, basis_head) = project_with_quest(&["Win the trial"]);
    project.meta.name.clear();
    let fixed_bytes = project.to_canonical_json().unwrap().len();
    let basis_len = MAX_PROJECT_JSON_BYTES - 1;
    assert!(fixed_bytes < basis_len);
    project.meta.name = "x".repeat(basis_len - fixed_bytes);
    assert_eq!(project.to_canonical_json().unwrap().len(), basis_len);

    let mut candidate = request(&project, &basis_head);
    candidate.title = "x".repeat(MAX_DRAFT_QUEST_TITLE_BYTES);
    assert!(matches!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV1::CandidateTooLarge {
            actual,
            limit: MAX_PROJECT_JSON_BYTES,
        } if actual > MAX_PROJECT_JSON_BYTES
    ));
}

#[test]
fn invalid_duplicate_and_over_limit_text_is_rejected() {
    let (project, basis_head) = project_with_quest(&["Stage one", "Stage two"]);
    let base = request(&project, &basis_head);

    for value in [
        String::new(),
        " leading".to_owned(),
        "trailing ".to_owned(),
        "line\nbreak".to_owned(),
        "x".repeat(MAX_REVISION3_QUEST_OUTLINE_EDIT_DISPLAY_NAME_BYTES_V1 + 1),
    ] {
        let mut candidate = base.clone();
        candidate.display_name = value;
        assert_eq!(
            rejected(&project, &basis_head, &candidate),
            Revision3QuestOutlineEditConflictV1::InvalidDisplayName
        );
    }

    for value in [
        String::new(),
        " leading".to_owned(),
        "Bad \" title".to_owned(),
        "x".repeat(MAX_DRAFT_QUEST_TITLE_BYTES + 1),
    ] {
        let mut candidate = base.clone();
        candidate.title = value;
        assert!(matches!(
            rejected(&project, &basis_head, &candidate),
            Revision3QuestOutlineEditConflictV1::InvalidOutlineText { .. }
        ));
    }

    let mut candidate = base.clone();
    candidate.objective_titles = vec!["Same".to_owned(), "same".to_owned()];
    assert!(matches!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV1::InvalidObjectiveTitles {
            error: DraftQuestSkeletonError::DuplicateObjectiveTitle {
                first: 1,
                second: 2,
            }
        }
    ));
    candidate = base.clone();
    candidate.objective_titles[1] = "Bad \\ objective".to_owned();
    assert!(matches!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV1::InvalidObjectiveTitles {
            error: DraftQuestSkeletonError::InvalidCharacter {
                field: DraftQuestField::AdditionalObjectiveTitle { index: 1 },
                ..
            }
        }
    ));
    candidate = base;
    candidate.objective_titles[0] = "x".repeat(MAX_DRAFT_QUEST_OBJECTIVE_TITLE_BYTES + 1);
    assert!(matches!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV1::InvalidObjectiveTitles {
            error: DraftQuestSkeletonError::ValueTooLong { .. }
        }
    ));
}

#[test]
fn project_quest_and_module_revision_overflows_are_rejected() {
    let (project, basis_head) = project_with_quest(&["Win the trial"]);

    let mut overflow = project.clone();
    overflow.revision = u64::MAX;
    let candidate = request(&overflow, &basis_head);
    assert_eq!(
        rejected(&overflow, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV1::ProjectRevisionOverflow
    );

    overflow = project.clone();
    overflow.entities.get_mut(&id(0x21)).unwrap().revision = u64::MAX;
    let candidate = request(&overflow, &basis_head);
    assert_eq!(
        rejected(&overflow, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV1::QuestRevisionOverflow { quest: id(0x21) }
    );

    overflow = project;
    overflow.entities.get_mut(&id(0x22)).unwrap().revision = u64::MAX;
    let candidate = request(&overflow, &basis_head);
    assert_eq!(
        rejected(&overflow, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV1::ScriptModuleRevisionOverflow { module: id(0x22) }
    );
}

#[test]
fn deterministic_owned_module_drift_is_rejected() {
    let (mut project, basis_head) = project_with_quest(&["Win the trial"]);
    let Revision3EntityPayload::ScriptModule(module) =
        &mut project.entities.get_mut(&id(0x22)).unwrap().payload
    else {
        panic!("fixture module kind")
    };
    module.source.push(' ');
    module.source_sha256 =
        Sha256Digest::from_bytes(Sha256::digest(module.source.as_bytes()).into());
    project.validate_closed_model().unwrap();
    let candidate = request(&project, &basis_head);
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestOutlineEditConflictV1::OwnedModuleDrift {
            quest: id(0x21),
            module: id(0x22),
        }
    );
}

#[test]
fn invalid_base_or_module_ownership_returns_no_candidate() {
    let (project, basis_head) = project_with_quest(&["Win the trial"]);
    let request_json = request(&project, &basis_head).to_canonical_json().unwrap();

    let mut missing_asset = project.clone();
    missing_asset.asset_store.assets.clear();
    let raw = serde_json::to_string(&missing_asset).unwrap();
    assert!(matches!(
        apply_revision3_quest_outline_edit_transaction_v1(&basis_head, &raw, &request_json),
        Err(Revision3QuestOutlineEditErrorV1::InvalidProject(_))
    ));

    let mut wrong_owner = project.clone();
    let Revision3EntityPayload::ScriptModule(module) =
        &mut wrong_owner.entities.get_mut(&id(0x22)).unwrap().payload
    else {
        panic!("fixture module kind")
    };
    module.owner.id = id(0x30);
    let raw = serde_json::to_string(&wrong_owner).unwrap();
    assert!(matches!(
        apply_revision3_quest_outline_edit_transaction_v1(&basis_head, &raw, &request_json),
        Err(Revision3QuestOutlineEditErrorV1::InvalidProject(_))
    ));

    let mut wrong_kind = project;
    let Revision3EntityPayload::QuestDraft(quest) =
        &mut wrong_kind.entities.get_mut(&id(0x21)).unwrap().payload
    else {
        panic!("fixture Quest kind")
    };
    quest.script_module.expected_kind = Revision3EntityKind::VoiceTake;
    let raw = serde_json::to_string(&wrong_kind).unwrap();
    assert!(matches!(
        apply_revision3_quest_outline_edit_transaction_v1(&basis_head, &raw, &request_json),
        Err(Revision3QuestOutlineEditErrorV1::InvalidProject(_))
    ));
}

#[test]
fn malformed_or_noncanonical_project_and_request_return_no_candidate() {
    let (project, basis_head) = project_with_quest(&["Win the trial"]);
    let request = request(&project, &basis_head).to_canonical_json().unwrap();
    let project_json = project.to_canonical_json().unwrap();
    assert!(matches!(
        apply_revision3_quest_outline_edit_transaction_v1(
            &basis_head,
            &format!(" {project_json}"),
            &request,
        ),
        Err(Revision3QuestOutlineEditErrorV1::InvalidProject(_))
    ));
    assert!(matches!(
        apply_revision3_quest_outline_edit_transaction_v1(
            &basis_head,
            &project_json,
            &format!(" {request}"),
        ),
        Err(Revision3QuestOutlineEditErrorV1::InvalidRequest(_))
    ));
}

// Keep the public payload alias in this integration test: changing the module's closed payload
// shape must remain a compile-time-visible API break rather than silently weakening the checks.
fn _script_module_shape(module: &Revision3ScriptModule) -> (&str, &str) {
    (&module.module_namespace, &module.module_relative_path)
}
