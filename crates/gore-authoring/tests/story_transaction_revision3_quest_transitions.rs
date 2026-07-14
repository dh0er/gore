use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::{
    apply_revision3_quest_transition_plan_transaction_v1, regenerate_revision3_quest_module_v2,
    revision3_quest_transition_plan_basis_v1, revision3_quest_transition_plan_seal_v1, AssetMeta,
    AssetStoreIndex, ContentSeal, EntityId, FormatV2, GameGenerationAnchor, ProjectId, ProjectMeta,
    ProjectRevision3, QuestCollisionArtifactRef, QuestCollisionCatalogInput,
    QuestTransitionConditionAtomV1, QuestTransitionConditionGroupV1, QuestTransitionEdgeV1,
    QuestTransitionNodeV1, QuestTransitionPlanV1, QuestTransitionPredicateV1,
    QuestTransitionStateTestV1, Revision2LocalizationEntry, Revision3Entity, Revision3EntityKind,
    Revision3EntityPayload, Revision3OriginRef, Revision3QuestDraft, Revision3QuestDraftInput,
    Revision3QuestGiverInput, Revision3QuestParentInput,
    Revision3QuestTransitionPlanEditBuildStatusV1, Revision3QuestTransitionPlanEditConflictV1,
    Revision3QuestTransitionPlanEditErrorV1, Revision3QuestTransitionPlanEditEvaluationV1,
    Revision3QuestTransitionPlanEditPublicationStatusV1,
    Revision3QuestTransitionPlanEditRequestJsonErrorV1, Revision3QuestTransitionPlanEditRequestV1,
    Revision3QuestTransitionPlanEditRuntimeStatusV1, Revision3ScriptModule, Revision3TypedRef,
    SchemaRevisionV3, Sha256Digest, WorkingHead, WorkingStoreFormat, MAX_PROJECT_JSON_BYTES,
    MAX_REVISION3_QUEST_TRANSITION_PLAN_EDIT_REQUEST_JSON_BYTES_V1,
    QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2, QUEST_COLLISION_CATALOG_LAYER_V2,
    REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION, REVISION3_QUEST_GENERATOR_ID,
    REVISION3_QUEST_GENERATOR_VERSION, REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION,
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
    let project_id = project_id(0x11);
    let quest_id = id(0x21);
    let module_id = id(0x22);
    let unrelated_id = id(0x30);
    let generation = target(0x31);
    let artifact = seal(0x41, 8192);
    let extra_asset = seal(0x44, 77);
    let artifact_ref = QuestCollisionArtifactRef {
        generation: generation.clone(),
        catalog_layer: QUEST_COLLISION_CATALOG_LAYER_V2.to_owned(),
        artifact: artifact.clone(),
        source_seal: seal(0x42, artifact.byte_len),
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
                .map(|title| (*title).to_owned())
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
        (
            unrelated_id,
            Revision3Entity {
                id: unrelated_id,
                display_name: "Unrelated localization".to_owned(),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: "GORE_UNRELATED_LOC".to_owned(),
                },
                revision: 9,
                payload: Revision3EntityPayload::LocalizationEntry(Revision2LocalizationEntry {
                    loc_id: "GORE_UNRELATED_LOC".to_owned(),
                    texts: BTreeMap::new(),
                }),
            },
        ),
    ]);
    let project = ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id,
        revision: 7,
        meta: ProjectMeta {
            name: "Quest transition edit tests".to_owned(),
            version: "1.0.0".to_owned(),
            author: "tests".to_owned(),
        },
        target: generation,
        authoring_locales: BTreeSet::new(),
        entities,
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

fn request(
    project: &ProjectRevision3,
    basis_head: &WorkingHead,
    transition_plan: QuestTransitionPlanV1,
) -> Revision3QuestTransitionPlanEditRequestV1 {
    let quest_id = id(0x21);
    let quest_entity = &project.entities[&quest_id];
    let basis = revision3_quest_transition_plan_basis_v1(quest(project)).unwrap();
    Revision3QuestTransitionPlanEditRequestV1 {
        expected_head: basis_head.clone(),
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        expected_target: project.target.clone(),
        quest_id,
        expected_quest_revision: quest_entity.revision,
        expected_transition_plan_seal: basis.seal,
        transition_plan,
    }
}

fn seed_request(
    project: &ProjectRevision3,
    basis_head: &WorkingHead,
) -> Revision3QuestTransitionPlanEditRequestV1 {
    let basis = revision3_quest_transition_plan_basis_v1(quest(project)).unwrap();
    assert!(basis.legacy_synthetic);
    request(project, basis_head, basis.plan)
}

fn evaluate(
    project: &ProjectRevision3,
    basis_head: &WorkingHead,
    request: &Revision3QuestTransitionPlanEditRequestV1,
) -> Result<Revision3QuestTransitionPlanEditEvaluationV1, Revision3QuestTransitionPlanEditErrorV1> {
    apply_revision3_quest_transition_plan_transaction_v1(
        basis_head,
        &project.to_canonical_json().unwrap(),
        &request.to_canonical_json().unwrap(),
    )
}

fn applied(
    project: &ProjectRevision3,
    basis_head: &WorkingHead,
    request: &Revision3QuestTransitionPlanEditRequestV1,
) -> Box<gore_authoring::Revision3QuestTransitionPlanEditOutcomeV1> {
    match evaluate(project, basis_head, request).unwrap() {
        Revision3QuestTransitionPlanEditEvaluationV1::Applied(outcome) => outcome,
        Revision3QuestTransitionPlanEditEvaluationV1::Rejected(rejection) => {
            panic!("unexpected rejection: {}", rejection.conflict)
        }
    }
}

fn rejected(
    project: &ProjectRevision3,
    basis_head: &WorkingHead,
    request: &Revision3QuestTransitionPlanEditRequestV1,
) -> Revision3QuestTransitionPlanEditConflictV1 {
    match evaluate(project, basis_head, request).unwrap() {
        Revision3QuestTransitionPlanEditEvaluationV1::Rejected(rejection) => rejection.conflict,
        Revision3QuestTransitionPlanEditEvaluationV1::Applied(_) => panic!("expected rejection"),
    }
}

fn automatic_root_start(plan: &mut QuestTransitionPlanV1) {
    let transition = plan
        .transitions
        .iter_mut()
        .find(|transition| {
            transition.node == QuestTransitionNodeV1::Root
                && transition.edge == QuestTransitionEdgeV1::Start
        })
        .expect("seed root start");
    transition.external_allowed = false;
    transition.predicate = Some(QuestTransitionPredicateV1 {
        any_of: vec![QuestTransitionConditionGroupV1 {
            all_of: vec![QuestTransitionConditionAtomV1 {
                node: QuestTransitionNodeV1::Root,
                test: QuestTransitionStateTestV1::Available,
                negated: false,
            }],
        }],
    });
}

#[test]
fn legacy_v2_and_v3_seed_upgrade_preserves_everything_except_plan_contract_and_three_revisions() {
    for objective_titles in [
        vec!["Win the trial"],
        vec!["Enter the arena", "Defeat the guard", "Report to Asghan"],
    ] {
        let (project, basis_head) = project_with_quest(&objective_titles);
        let before = project.clone();
        let old_quest = quest(&before).clone();
        let old_module_entity = before.entities[&id(0x22)].clone();
        let request = seed_request(&project, &basis_head);
        let outcome = applied(&project, &basis_head, &request);

        assert_eq!(outcome.basis_head, basis_head);
        assert_eq!(outcome.quest_id, id(0x21));
        assert_eq!(outcome.script_module_id, id(0x22));
        assert_eq!(
            outcome.previous_generator_version,
            old_quest.generator_version
        );
        assert!(outcome.upgraded_from_legacy);
        assert_eq!(outcome.quest_revision, 4);
        assert_eq!(outcome.script_module_revision, 6);
        assert_eq!(
            outcome.build_status,
            Revision3QuestTransitionPlanEditBuildStatusV1::Blocked
        );
        assert_eq!(
            outcome.runtime_status,
            Revision3QuestTransitionPlanEditRuntimeStatusV1::RuntimeUnqualified
        );
        assert_eq!(
            outcome.publication_status,
            Revision3QuestTransitionPlanEditPublicationStatusV1::NotSupported
        );
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

        let after_quest_entity = &outcome.project.entities[&id(0x21)];
        assert_eq!(after_quest_entity.id, before.entities[&id(0x21)].id);
        assert_eq!(
            after_quest_entity.display_name,
            before.entities[&id(0x21)].display_name
        );
        assert_eq!(after_quest_entity.origin, before.entities[&id(0x21)].origin);
        assert_eq!(
            after_quest_entity.revision,
            before.entities[&id(0x21)].revision + 1
        );
        let after_quest = quest(&outcome.project);
        let mut expected_quest = old_quest;
        expected_quest.generator_version = REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION;
        expected_quest.input.transition_plan = Some(Box::new(request.transition_plan.clone()));
        assert_eq!(after_quest, &expected_quest);

        let after_module_entity = &outcome.project.entities[&id(0x22)];
        assert_eq!(after_module_entity.id, old_module_entity.id);
        assert_eq!(
            after_module_entity.display_name,
            old_module_entity.display_name
        );
        assert_eq!(after_module_entity.revision, old_module_entity.revision + 1);
        let Revision3OriginRef::Generated {
            generator_id,
            generator_version,
            owner,
        } = &after_module_entity.origin
        else {
            panic!("generated module origin")
        };
        let Revision3OriginRef::Generated {
            generator_id: old_generator_id,
            owner: old_owner,
            ..
        } = &old_module_entity.origin
        else {
            panic!("old generated module origin")
        };
        assert_eq!(generator_id, old_generator_id);
        assert_eq!(
            *generator_version,
            REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION
        );
        assert_eq!(owner, old_owner);
        let Revision3EntityPayload::ScriptModule(old_module) = &old_module_entity.payload else {
            panic!("old module kind")
        };
        let Revision3EntityPayload::ScriptModule(after_module) = &after_module_entity.payload
        else {
            panic!("edited module kind")
        };
        assert_eq!(after_module.owner, old_module.owner);
        assert_eq!(after_module.generator_id, old_module.generator_id);
        assert_eq!(
            after_module.generator_version,
            REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION
        );
        assert_eq!(after_module.module_namespace, old_module.module_namespace);
        assert_eq!(
            after_module.module_relative_path,
            old_module.module_relative_path
        );
        assert_eq!(after_module.status, old_module.status);
        assert_eq!(
            after_module,
            &regenerate_revision3_quest_module_v2(after_quest, collision_input(after_quest))
                .unwrap()
        );
        assert_eq!(
            outcome.transition_plan_seal,
            revision3_quest_transition_plan_seal_v1(&request.transition_plan).unwrap()
        );

        let reopened = ProjectRevision3::from_json(&outcome.canonical_project_json).unwrap();
        assert_eq!(reopened, outcome.project);
        assert_eq!(
            outcome.canonical_project_json,
            outcome.project.to_canonical_json().unwrap()
        );
    }
}

#[test]
fn v4_edit_uses_retained_plan_cas_and_same_plan_is_a_no_op() {
    let (legacy, first_head) = project_with_quest(&["Win the trial"]);
    let upgraded = applied(&legacy, &first_head, &seed_request(&legacy, &first_head));
    let v4 = upgraded.project.clone();
    let second_head = head(0x62);
    let basis = revision3_quest_transition_plan_basis_v1(quest(&v4)).unwrap();
    assert!(!basis.legacy_synthetic);
    assert_eq!(
        basis.generator_version,
        REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION
    );

    let no_op = request(&v4, &second_head, basis.plan.clone());
    assert_eq!(
        rejected(&v4, &second_head, &no_op),
        Revision3QuestTransitionPlanEditConflictV1::NoChanges
    );

    let mut edited_plan = basis.plan;
    automatic_root_start(&mut edited_plan);
    let edit = request(&v4, &second_head, edited_plan.clone());
    let outcome = applied(&v4, &second_head, &edit);
    assert!(!outcome.upgraded_from_legacy);
    assert_eq!(
        outcome.previous_generator_version,
        REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION
    );
    assert_eq!(outcome.project.revision, v4.revision + 1);
    assert_eq!(outcome.quest_revision, v4.entities[&id(0x21)].revision + 1);
    assert_eq!(
        outcome.script_module_revision,
        v4.entities[&id(0x22)].revision + 1
    );
    assert_eq!(
        quest(&outcome.project).input.transition_plan.as_deref(),
        Some(&edited_plan)
    );
    assert_ne!(
        outcome.transition_plan_seal,
        edit.expected_transition_plan_seal
    );
    assert_eq!(outcome.project.asset_store, v4.asset_store);
    assert_eq!(outcome.project.entities[&id(0x30)], v4.entities[&id(0x30)]);
}

#[test]
fn transition_only_edit_preserves_active_slots_and_never_reuses_burned_ordinals() {
    let (legacy_multi, first_head) = project_with_quest(&["Enter the arena", "Report to Asghan"]);
    let upgraded_multi = applied(
        &legacy_multi,
        &first_head,
        &seed_request(&legacy_multi, &first_head),
    );
    let v4_multi = upgraded_multi.project;
    let second_head = head(0x63);
    let mut replaced_slot = revision3_quest_transition_plan_basis_v1(quest(&v4_multi))
        .unwrap()
        .plan;
    replaced_slot.objective_slots[1] = 3;
    replaced_slot.objective_order[1] = 3;
    replaced_slot.next_slot_ordinal = 4;
    for transition in &mut replaced_slot.transitions {
        if transition.node == (QuestTransitionNodeV1::Objective { slot: 2 }) {
            transition.node = QuestTransitionNodeV1::Objective { slot: 3 };
        }
    }
    let slot_request = request(&v4_multi, &second_head, replaced_slot);
    assert_eq!(
        rejected(&v4_multi, &second_head, &slot_request),
        Revision3QuestTransitionPlanEditConflictV1::ObjectiveSlotsChanged
    );

    let (legacy_single, third_head) = project_with_quest(&["Win the trial"]);
    let mut burn_plan = revision3_quest_transition_plan_basis_v1(quest(&legacy_single))
        .unwrap()
        .plan;
    burn_plan.next_slot_ordinal = 10;
    let burned = applied(
        &legacy_single,
        &third_head,
        &request(&legacy_single, &third_head, burn_plan),
    );
    let burned_v4 = burned.project;
    let fourth_head = head(0x64);
    let mut regressed = revision3_quest_transition_plan_basis_v1(quest(&burned_v4))
        .unwrap()
        .plan;
    regressed.next_slot_ordinal = 2;
    let request = request(&burned_v4, &fourth_head, regressed);
    assert_eq!(
        rejected(&burned_v4, &fourth_head, &request),
        Revision3QuestTransitionPlanEditConflictV1::NextSlotOrdinalRegression {
            current: 10,
            requested: 2,
        }
    );
}

#[test]
fn exact_head_project_quest_and_plan_cas_conflicts_fail_closed() {
    let (project, basis_head) = project_with_quest(&["Win the trial"]);
    let base = seed_request(&project, &basis_head);

    let mut candidate = base.clone();
    candidate.expected_head = head(0x72);
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestTransitionPlanEditConflictV1::CurrentHeadMismatch
    );
    candidate = base.clone();
    candidate.expected_project_id = project_id(0x12);
    assert!(matches!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestTransitionPlanEditConflictV1::ProjectIdentityMismatch { .. }
    ));
    candidate = base.clone();
    candidate.expected_revision -= 1;
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestTransitionPlanEditConflictV1::ProjectRevisionConflict {
            expected: 6,
            actual: 7,
        }
    );
    candidate = base.clone();
    candidate.expected_target = target(0x32);
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestTransitionPlanEditConflictV1::ProjectTargetMismatch
    );
    candidate = base.clone();
    candidate.expected_quest_revision -= 1;
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestTransitionPlanEditConflictV1::QuestRevisionConflict {
            expected: 2,
            actual: 3,
        }
    );
    candidate = base.clone();
    candidate.expected_transition_plan_seal.sha256 = Sha256Digest::from_bytes([0xee; 32]);
    assert!(matches!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestTransitionPlanEditConflictV1::TransitionPlanSealConflict { .. }
    ));
    candidate = base.clone();
    candidate.quest_id = EntityId::from_bytes([0; 16]);
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestTransitionPlanEditConflictV1::ZeroQuestId
    );
    candidate = base;
    candidate.quest_id = id(0x30);
    candidate.expected_quest_revision = 9;
    assert_eq!(
        rejected(&project, &basis_head, &candidate),
        Revision3QuestTransitionPlanEditConflictV1::InvalidQuestEntity { quest: id(0x30) }
    );
}

#[test]
fn invalid_plan_module_drift_and_all_revision_overflows_are_rejected() {
    let (project, basis_head) = project_with_quest(&["Win the trial"]);
    let mut invalid = seed_request(&project, &basis_head);
    invalid.transition_plan.objective_order.clear();
    assert!(matches!(
        rejected(&project, &basis_head, &invalid),
        Revision3QuestTransitionPlanEditConflictV1::InvalidTransitionPlan { .. }
    ));

    let mut drift = project.clone();
    let Revision3EntityPayload::ScriptModule(module) =
        &mut drift.entities.get_mut(&id(0x22)).unwrap().payload
    else {
        panic!("fixture module kind")
    };
    module.source.push(' ');
    module.source_sha256 =
        Sha256Digest::from_bytes(Sha256::digest(module.source.as_bytes()).into());
    drift.validate_closed_model().unwrap();
    let drift_request = seed_request(&drift, &basis_head);
    assert_eq!(
        rejected(&drift, &basis_head, &drift_request),
        Revision3QuestTransitionPlanEditConflictV1::OwnedModuleDrift {
            quest: id(0x21),
            module: id(0x22),
        }
    );

    let mut overflow = project.clone();
    overflow.revision = u64::MAX;
    let request = seed_request(&overflow, &basis_head);
    assert_eq!(
        rejected(&overflow, &basis_head, &request),
        Revision3QuestTransitionPlanEditConflictV1::ProjectRevisionOverflow
    );
    overflow = project.clone();
    overflow.entities.get_mut(&id(0x21)).unwrap().revision = u64::MAX;
    let request = seed_request(&overflow, &basis_head);
    assert_eq!(
        rejected(&overflow, &basis_head, &request),
        Revision3QuestTransitionPlanEditConflictV1::QuestRevisionOverflow { quest: id(0x21) }
    );
    overflow = project;
    overflow.entities.get_mut(&id(0x22)).unwrap().revision = u64::MAX;
    let request = seed_request(&overflow, &basis_head);
    assert_eq!(
        rejected(&overflow, &basis_head, &request),
        Revision3QuestTransitionPlanEditConflictV1::ScriptModuleRevisionOverflow {
            module: id(0x22),
        }
    );
}

#[test]
fn request_and_plan_seals_are_bounded_duplicate_free_canonical_and_domain_separated() {
    let (project, basis_head) = project_with_quest(&["Win the trial"]);
    let request = seed_request(&project, &basis_head);
    let json = request.to_canonical_json().unwrap();
    assert_eq!(
        Revision3QuestTransitionPlanEditRequestV1::from_json(&json).unwrap(),
        request
    );
    assert!(matches!(
        Revision3QuestTransitionPlanEditRequestV1::from_json(&format!(" {json}")),
        Err(Revision3QuestTransitionPlanEditRequestJsonErrorV1::NonCanonicalJson)
    ));
    let duplicate = json.replacen(
        "\"expected_revision\":7",
        "\"expected_revision\":7,\"expected_revision\":7",
        1,
    );
    assert!(matches!(
        Revision3QuestTransitionPlanEditRequestV1::from_json(&duplicate),
        Err(Revision3QuestTransitionPlanEditRequestJsonErrorV1::InvalidJson(_))
    ));
    let unknown = json.replacen('{', "{\"unknown\":false,", 1);
    assert!(matches!(
        Revision3QuestTransitionPlanEditRequestV1::from_json(&unknown),
        Err(Revision3QuestTransitionPlanEditRequestJsonErrorV1::InvalidJson(_))
    ));
    let oversized_raw = format!(
        "{{\"padding\":\"{}\"}}",
        "x".repeat(MAX_REVISION3_QUEST_TRANSITION_PLAN_EDIT_REQUEST_JSON_BYTES_V1)
    );
    assert!(matches!(
        Revision3QuestTransitionPlanEditRequestV1::from_json(&oversized_raw),
        Err(Revision3QuestTransitionPlanEditRequestJsonErrorV1::InputTooLarge { .. })
    ));
    let mut oversized = request.clone();
    let transition = oversized.transition_plan.transitions[0].clone();
    oversized.transition_plan.transitions =
        vec![transition; MAX_REVISION3_QUEST_TRANSITION_PLAN_EDIT_REQUEST_JSON_BYTES_V1 / 8];
    assert!(matches!(
        oversized.to_canonical_json(),
        Err(Revision3QuestTransitionPlanEditRequestJsonErrorV1::InputTooLarge { .. })
    ));

    let plan_json = serde_json::to_vec(&request.transition_plan).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(b"gore-authoring.revision3-quest-transition-plan-v1\0");
    hasher.update((plan_json.len() as u64).to_be_bytes());
    hasher.update(&plan_json);
    assert_eq!(
        revision3_quest_transition_plan_seal_v1(&request.transition_plan).unwrap(),
        ContentSeal {
            byte_len: plan_json.len() as u64,
            sha256: Sha256Digest::from_bytes(hasher.finalize().into()),
        }
    );
}

#[test]
fn malformed_noncanonical_and_over_capacity_projects_never_return_a_candidate() {
    let (project, basis_head) = project_with_quest(&["Win the trial"]);
    let request = seed_request(&project, &basis_head)
        .to_canonical_json()
        .unwrap();
    let project_json = project.to_canonical_json().unwrap();
    assert!(matches!(
        apply_revision3_quest_transition_plan_transaction_v1(
            &basis_head,
            &format!(" {project_json}"),
            &request,
        ),
        Err(Revision3QuestTransitionPlanEditErrorV1::InvalidProject(_))
    ));
    assert!(matches!(
        apply_revision3_quest_transition_plan_transaction_v1(
            &basis_head,
            &project_json,
            &format!(" {request}"),
        ),
        Err(Revision3QuestTransitionPlanEditErrorV1::InvalidRequest(_))
    ));

    let mut full = project;
    full.meta.name.clear();
    let fixed_bytes = full.to_canonical_json().unwrap().len();
    let basis_len = MAX_PROJECT_JSON_BYTES - 1;
    assert!(fixed_bytes < basis_len);
    full.meta.name = "x".repeat(basis_len - fixed_bytes);
    assert_eq!(full.to_canonical_json().unwrap().len(), basis_len);
    let request = seed_request(&full, &basis_head);
    assert!(matches!(
        rejected(&full, &basis_head, &request),
        Revision3QuestTransitionPlanEditConflictV1::CandidateTooLarge {
            actual,
            limit: MAX_PROJECT_JSON_BYTES,
        } if actual > MAX_PROJECT_JSON_BYTES
    ));
}

// Keep the public payload alias in this integration test: changing the module's closed payload
// shape must remain a compile-time-visible API break rather than weakening the preservation proof.
fn _script_module_shape(module: &Revision3ScriptModule) -> (&str, &str) {
    (&module.module_namespace, &module.module_relative_path)
}
