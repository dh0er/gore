use gore_authoring::{
    validate_draft_quest_transition_plan_v1, CatalogQualifiedParentQuest,
    CatalogQualifiedQuestGiver, ContentSeal, DraftQuestCollisionCatalog, DraftQuestSkeletonError,
    DraftQuestSkeletonInput, DraftQuestSkeletonInputV2, DraftQuestSkeletonInputV3,
    DraftQuestSkeletonV1, DraftQuestSkeletonV2, DraftQuestSkeletonV3, EntityId,
    GameGenerationAnchor, QuestTransitionConditionAtomV1, QuestTransitionConditionGroupV1,
    QuestTransitionEdgeV1, QuestTransitionEffectKindV1, QuestTransitionEffectV1,
    QuestTransitionNodeV1, QuestTransitionPlanV1, QuestTransitionPredicateV1,
    QuestTransitionStateTestV1, Sha256Digest, MAX_QUEST_TRANSITION_EFFECTS_V1,
    MAX_QUEST_TRANSITION_PREDICATE_GROUPS_V1,
};

fn seal(byte: u8, byte_len: u64) -> ContentSeal {
    ContentSeal {
        byte_len,
        sha256: Sha256Digest::from_bytes([byte; 32]),
    }
}

fn target() -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: seal(1, 1_000_000),
    }
}

fn input() -> DraftQuestSkeletonInput {
    let target = target();
    DraftQuestSkeletonInput {
        target: target.clone(),
        quest_id: EntityId::from_bytes([2; 16]),
        module_namespace: "GoreMods.Tests.SemanticQuest".into(),
        technical_id: "GORE_SEMANTIC_QUEST".into(),
        text_helper: "GoreSemanticQuestText".into(),
        parent_quest: CatalogQualifiedParentQuest::new(
            target.clone(),
            seal(3, 10),
            "base-game.quests",
            "CatalogQuest_Parent",
            "UQuest_Parent",
        )
        .unwrap(),
        giver: CatalogQualifiedQuestGiver::new(
            target.clone(),
            seal(4, 10),
            "base-game.characters",
            "CatalogCharacter_Asghan",
            "OM_GRD_Asghan_263",
        )
        .unwrap(),
        title: "A semantic quest".into(),
        description: "Exercise the bounded transition renderer.".into(),
        objective_title: "First authored title".into(),
        collision_catalog: DraftQuestCollisionCatalog::new(
            target,
            seal(5, 10),
            "resolved.scripts",
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .unwrap(),
    }
}

fn node(slot: u16) -> QuestTransitionNodeV1 {
    QuestTransitionNodeV1::Objective { slot }
}

fn predicate(
    node: QuestTransitionNodeV1,
    test: QuestTransitionStateTestV1,
) -> QuestTransitionPredicateV1 {
    QuestTransitionPredicateV1 {
        any_of: vec![QuestTransitionConditionGroupV1 {
            all_of: vec![QuestTransitionConditionAtomV1 {
                node,
                test,
                negated: false,
            }],
        }],
    }
}

fn transition_mut(
    plan: &mut QuestTransitionPlanV1,
    node: QuestTransitionNodeV1,
    edge: QuestTransitionEdgeV1,
) -> &mut gore_authoring::QuestTransitionV1 {
    plan.transitions
        .iter_mut()
        .find(|transition| transition.node == node && transition.edge == edge)
        .unwrap()
}

fn assert_invalid(plan: &QuestTransitionPlanV1) {
    assert!(matches!(
        validate_draft_quest_transition_plan_v1(plan, plan.objective_slots.len()),
        Err(DraftQuestSkeletonError::InvalidTransitionPlan { .. })
    ));
}

#[test]
fn legacy_seed_has_stable_wire_and_reproduces_frozen_source_bytes() {
    let one = QuestTransitionPlanV1::legacy_seed(1).unwrap();
    assert_eq!(
        serde_json::to_string(&one).unwrap(),
        r#"{"objective_slots":[1],"objective_order":[1],"next_slot_ordinal":2,"transitions":[{"node":{"kind":"root"},"edge":"availability","external_allowed":true},{"node":{"kind":"root"},"edge":"start","external_allowed":true},{"node":{"kind":"objective","slot":1},"edge":"availability","external_allowed":true},{"node":{"kind":"objective","slot":1},"edge":"start","external_allowed":true},{"node":{"kind":"objective","slot":1},"edge":"success","external_allowed":true,"succeeds_parent":true}]}"#
    );
    let frozen_one = DraftQuestSkeletonV1::new(input()).unwrap().generate();
    let semantic_one = DraftQuestSkeletonV3::new(DraftQuestSkeletonInputV3 {
        base: input(),
        additional_objective_titles: Vec::new(),
        transition_plan: one,
    })
    .unwrap()
    .generate();
    assert_eq!(semantic_one.source, frozen_one.source);
    assert_eq!(semantic_one.source_sha256, frozen_one.source_sha256);

    let titles = vec![
        "Second authored title".to_owned(),
        "Third authored title".to_owned(),
    ];
    let frozen_three = DraftQuestSkeletonV2::new(DraftQuestSkeletonInputV2 {
        base: input(),
        additional_objective_titles: titles.clone(),
    })
    .unwrap()
    .generate();
    let semantic_three = DraftQuestSkeletonV3::new(DraftQuestSkeletonInputV3 {
        base: input(),
        additional_objective_titles: titles,
        transition_plan: QuestTransitionPlanV1::legacy_seed(3).unwrap(),
    })
    .unwrap()
    .generate();
    assert_eq!(semantic_three.source, frozen_three.source);
    assert_eq!(semantic_three.source_sha256, frozen_three.source_sha256);
}

#[test]
fn presentation_reorder_preserves_stable_slot_classes_and_getters() {
    let mut plan = QuestTransitionPlanV1::legacy_seed(2).unwrap();
    plan.objective_order = vec![2, 1];
    let generated = DraftQuestSkeletonV3::new(DraftQuestSkeletonInputV3 {
        base: input(),
        additional_objective_titles: vec!["Second authored title".into()],
        transition_plan: plan,
    })
    .unwrap()
    .generate();

    let slot_two = generated
        .source
        .find("class UQuest_GORE_SEMANTIC_QUEST_OBJ_2")
        .unwrap();
    let slot_one = generated
        .source
        .find("class UQuest_GORE_SEMANTIC_QUEST_OBJ_DONE")
        .unwrap();
    assert!(slot_two < slot_one);
    assert!(generated.source[slot_two..slot_one]
        .contains("GoreSemanticQuestText(n\"First authored title\")"));
    assert!(
        generated.source[slot_one..].contains("GoreSemanticQuestText(n\"Second authored title\")")
    );
    assert!(generated
        .source
        .contains("UQuest_GORE_SEMANTIC_QUEST_OBJ_DONE GetGoreSemanticQuestObjective()"));
    assert!(generated
        .source
        .contains("UQuest_GORE_SEMANTIC_QUEST_OBJ_2 GetGoreSemanticQuestObjective2()"));
}

#[test]
fn predicates_external_overlap_and_effects_lower_to_exact_guarded_hooks() {
    let mut plan = QuestTransitionPlanV1::legacy_seed(2).unwrap();
    transition_mut(
        &mut plan,
        QuestTransitionNodeV1::Root,
        QuestTransitionEdgeV1::Start,
    )
    .effects = vec![QuestTransitionEffectV1 {
        target: node(2),
        effect: QuestTransitionEffectKindV1::Start,
    }];
    let availability = transition_mut(&mut plan, node(1), QuestTransitionEdgeV1::Availability);
    availability.external_allowed = false;
    availability.predicate = Some(predicate(
        QuestTransitionNodeV1::Root,
        QuestTransitionStateTestV1::Running,
    ));
    let success = transition_mut(&mut plan, node(1), QuestTransitionEdgeV1::Success);
    success.predicate = Some(predicate(
        QuestTransitionNodeV1::Root,
        QuestTransitionStateTestV1::Running,
    ));
    success.effects = vec![QuestTransitionEffectV1 {
        target: node(2),
        effect: QuestTransitionEffectKindV1::Succeed,
    }];
    plan.transitions.push(gore_authoring::QuestTransitionV1 {
        node: node(2),
        edge: QuestTransitionEdgeV1::Failure,
        external_allowed: true,
        predicate: Some(predicate(node(1), QuestTransitionStateTestV1::Failed)),
        effects: vec![QuestTransitionEffectV1 {
            target: QuestTransitionNodeV1::Root,
            effect: QuestTransitionEffectKindV1::Fail,
        }],
        succeeds_parent: false,
    });
    validate_draft_quest_transition_plan_v1(&plan, 2).unwrap();

    let source = DraftQuestSkeletonV3::new(DraftQuestSkeletonInputV3 {
        base: input(),
        additional_objective_titles: vec!["Second authored title".into()],
        transition_plan: plan,
    })
    .unwrap()
    .generate()
    .source;
    for hook in [
        "bool ShouldBeAvailable_Implementation()",
        "bool ShouldSucceed_Implementation()",
        "bool ShouldFail_Implementation()",
        "void HandleQuestStarted_Implementation()",
        "void HandleQuestSucceeded_Implementation()",
        "void HandleQuestFailed_Implementation()",
    ] {
        assert!(source.contains(hook), "missing hook {hook}");
    }
    // External success and an automatic predicate are independent and intentionally coexist.
    let objective_one = source
        .find("class UQuest_GORE_SEMANTIC_QUEST_OBJ_DONE")
        .unwrap();
    let objective_two = source
        .find("class UQuest_GORE_SEMANTIC_QUEST_OBJ_2")
        .unwrap();
    assert!(source[objective_one..objective_two].contains("bExternalSuccessTrigger = true"));
    assert!(source[objective_one..objective_two].contains("ShouldSucceed_Implementation"));
    assert!(source[objective_one..objective_two].contains("bExternalAvailabilityTrigger = false"));
    assert!(source[objective_two..].contains("bExternalFailTrigger = true"));
    assert!(source.contains("if (ObjectiveQuest2 != nullptr && !ObjectiveQuest2.HasBeenStarted())"));
    assert!(source.contains(
        "if (ObjectiveQuest2 != nullptr && ObjectiveQuest2.IsRunning())\n            ObjectiveQuest2.SucceedQuest(nullptr);"
    ));
    assert!(source.contains(
        "if (RootQuest != nullptr && RootQuest.IsRunning())\n            RootQuest.FailQuest(nullptr);"
    ));
}

#[test]
fn closed_validator_rejects_shape_driver_predicate_effect_and_terminal_conflicts() {
    let mut invalid = QuestTransitionPlanV1::legacy_seed(2).unwrap();
    invalid.objective_slots.swap(0, 1);
    assert_invalid(&invalid);

    let mut invalid = QuestTransitionPlanV1::legacy_seed(2).unwrap();
    invalid.objective_order = vec![1, 1];
    assert_invalid(&invalid);

    let mut invalid = QuestTransitionPlanV1::legacy_seed(2).unwrap();
    invalid.next_slot_ordinal = 2;
    assert_invalid(&invalid);

    let mut invalid = QuestTransitionPlanV1::legacy_seed(1).unwrap();
    invalid.transitions.push(invalid.transitions[0].clone());
    assert_invalid(&invalid);

    let mut invalid = QuestTransitionPlanV1::legacy_seed(1).unwrap();
    let start = transition_mut(
        &mut invalid,
        QuestTransitionNodeV1::Root,
        QuestTransitionEdgeV1::Start,
    );
    start.external_allowed = false;
    assert_invalid(&invalid);

    let mut invalid = QuestTransitionPlanV1::legacy_seed(1).unwrap();
    transition_mut(&mut invalid, node(1), QuestTransitionEdgeV1::Success).predicate =
        Some(QuestTransitionPredicateV1 {
            any_of: (0..=MAX_QUEST_TRANSITION_PREDICATE_GROUPS_V1)
                .map(|index| QuestTransitionConditionGroupV1 {
                    all_of: vec![QuestTransitionConditionAtomV1 {
                        node: QuestTransitionNodeV1::Root,
                        test: QuestTransitionStateTestV1::Running,
                        negated: index % 2 != 0,
                    }],
                })
                .collect(),
        });
    assert_invalid(&invalid);

    let mut invalid = QuestTransitionPlanV1::legacy_seed(1).unwrap();
    transition_mut(&mut invalid, node(1), QuestTransitionEdgeV1::Success).predicate =
        Some(QuestTransitionPredicateV1 {
            any_of: vec![QuestTransitionConditionGroupV1 {
                all_of: vec![
                    QuestTransitionConditionAtomV1 {
                        node: QuestTransitionNodeV1::Root,
                        test: QuestTransitionStateTestV1::Running,
                        negated: false,
                    },
                    QuestTransitionConditionAtomV1 {
                        node: QuestTransitionNodeV1::Root,
                        test: QuestTransitionStateTestV1::Running,
                        negated: true,
                    },
                ],
            }],
        });
    assert_invalid(&invalid);

    let mut invalid = QuestTransitionPlanV1::legacy_seed(1).unwrap();
    transition_mut(&mut invalid, node(1), QuestTransitionEdgeV1::Success).predicate =
        Some(predicate(node(9), QuestTransitionStateTestV1::Succeeded));
    assert_invalid(&invalid);

    let mut invalid = QuestTransitionPlanV1::legacy_seed(1).unwrap();
    transition_mut(&mut invalid, node(1), QuestTransitionEdgeV1::Success).effects =
        vec![QuestTransitionEffectV1 {
            target: node(1),
            effect: QuestTransitionEffectKindV1::Succeed,
        }];
    assert_invalid(&invalid);

    let mut invalid = QuestTransitionPlanV1::legacy_seed(1).unwrap();
    transition_mut(
        &mut invalid,
        QuestTransitionNodeV1::Root,
        QuestTransitionEdgeV1::Start,
    )
    .effects = vec![QuestTransitionEffectV1 {
        target: node(1),
        effect: QuestTransitionEffectKindV1::Start,
    }];
    transition_mut(&mut invalid, node(1), QuestTransitionEdgeV1::Start).effects =
        vec![QuestTransitionEffectV1 {
            target: QuestTransitionNodeV1::Root,
            effect: QuestTransitionEffectKindV1::Start,
        }];
    assert_invalid(&invalid);

    let mut invalid = QuestTransitionPlanV1::legacy_seed(1).unwrap();
    let shared = predicate(
        QuestTransitionNodeV1::Root,
        QuestTransitionStateTestV1::Running,
    );
    transition_mut(&mut invalid, node(1), QuestTransitionEdgeV1::Success).predicate =
        Some(shared.clone());
    invalid.transitions.push(gore_authoring::QuestTransitionV1 {
        node: node(1),
        edge: QuestTransitionEdgeV1::Failure,
        external_allowed: false,
        predicate: Some(shared),
        effects: Vec::new(),
        succeeds_parent: false,
    });
    assert_invalid(&invalid);

    let mut invalid = QuestTransitionPlanV1::legacy_seed(1).unwrap();
    transition_mut(
        &mut invalid,
        QuestTransitionNodeV1::Root,
        QuestTransitionEdgeV1::Start,
    )
    .succeeds_parent = true;
    assert_invalid(&invalid);

    for terminal in [
        QuestTransitionEffectKindV1::Succeed,
        QuestTransitionEffectKindV1::Fail,
    ] {
        let mut invalid = QuestTransitionPlanV1::legacy_seed(1).unwrap();
        transition_mut(&mut invalid, node(1), QuestTransitionEdgeV1::Success).effects =
            vec![QuestTransitionEffectV1 {
                target: QuestTransitionNodeV1::Root,
                effect: terminal,
            }];
        assert_invalid(&invalid);
    }

    let mut invalid = QuestTransitionPlanV1::legacy_seed(1).unwrap();
    invalid.transitions.push(gore_authoring::QuestTransitionV1 {
        node: QuestTransitionNodeV1::Root,
        edge: QuestTransitionEdgeV1::Success,
        external_allowed: true,
        predicate: None,
        effects: vec![QuestTransitionEffectV1 {
            target: node(1),
            effect: QuestTransitionEffectKindV1::Succeed,
        }],
        succeeds_parent: false,
    });
    invalid
        .transitions
        .sort_by_key(|transition| (transition.node, transition.edge));
    assert_invalid(&invalid);

    let mut invalid = QuestTransitionPlanV1::legacy_seed(8).unwrap();
    transition_mut(
        &mut invalid,
        QuestTransitionNodeV1::Root,
        QuestTransitionEdgeV1::Start,
    )
    .effects = (0..=MAX_QUEST_TRANSITION_EFFECTS_V1)
        .map(|index| QuestTransitionEffectV1 {
            target: node((index / 3 + 1) as u16),
            effect: match index % 3 {
                0 => QuestTransitionEffectKindV1::Start,
                1 => QuestTransitionEffectKindV1::Succeed,
                _ => QuestTransitionEffectKindV1::Fail,
            },
        })
        .collect();
    assert_invalid(&invalid);
}

#[test]
fn automatic_success_and_failure_predicates_must_be_provably_disjoint() {
    let mut overlapping = QuestTransitionPlanV1::legacy_seed(1).unwrap();
    transition_mut(&mut overlapping, node(1), QuestTransitionEdgeV1::Success).predicate =
        Some(predicate(
            QuestTransitionNodeV1::Root,
            QuestTransitionStateTestV1::Running,
        ));
    overlapping
        .transitions
        .push(gore_authoring::QuestTransitionV1 {
            node: node(1),
            edge: QuestTransitionEdgeV1::Failure,
            external_allowed: false,
            predicate: Some(QuestTransitionPredicateV1 {
                any_of: vec![QuestTransitionConditionGroupV1 {
                    all_of: vec![
                        QuestTransitionConditionAtomV1 {
                            node: QuestTransitionNodeV1::Root,
                            test: QuestTransitionStateTestV1::Running,
                            negated: false,
                        },
                        QuestTransitionConditionAtomV1 {
                            node: node(1),
                            test: QuestTransitionStateTestV1::Available,
                            negated: false,
                        },
                    ],
                }],
            }),
            effects: Vec::new(),
            succeeds_parent: false,
        });
    assert_invalid(&overlapping);

    let mut disjoint = QuestTransitionPlanV1::legacy_seed(1).unwrap();
    transition_mut(&mut disjoint, node(1), QuestTransitionEdgeV1::Success).predicate =
        Some(predicate(
            QuestTransitionNodeV1::Root,
            QuestTransitionStateTestV1::Succeeded,
        ));
    disjoint
        .transitions
        .push(gore_authoring::QuestTransitionV1 {
            node: node(1),
            edge: QuestTransitionEdgeV1::Failure,
            external_allowed: false,
            predicate: Some(predicate(
                QuestTransitionNodeV1::Root,
                QuestTransitionStateTestV1::Failed,
            )),
            effects: Vec::new(),
            succeeds_parent: false,
        });
    validate_draft_quest_transition_plan_v1(&disjoint, 1).unwrap();
}

#[test]
fn wire_is_closed_and_plan_presence_is_explicit() {
    let plan = QuestTransitionPlanV1::legacy_seed(1).unwrap();
    let mut value = serde_json::to_value(&plan).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("unknown".into(), serde_json::Value::Bool(true));
    assert!(serde_json::from_value::<QuestTransitionPlanV1>(value).is_err());
}
