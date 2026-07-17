use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::{
    apply_revision3_npc_profile_edit_transaction_v1, AssetMeta, AssetStoreIndex, ContentSeal,
    EntityId, FormatV2, GameGenerationAnchor, ProjectId, ProjectMeta, ProjectRevision3,
    Revision2DialogLine, Revision2LocalizationEntry, Revision2NpcDraftInput,
    Revision2NpcParentClassInput, Revision3Entity, Revision3EntityKind, Revision3EntityPayload,
    Revision3NpcCatalogSelectionV1, Revision3NpcDraft, Revision3NpcGreetingBindingV1,
    Revision3NpcProfileCatalogContextV1, Revision3NpcProfileEditBuildStatusV1,
    Revision3NpcProfileEditCatalogAuthorityV1, Revision3NpcProfileEditCollisionAuthorityV1,
    Revision3NpcProfileEditConflictV1, Revision3NpcProfileEditErrorV1,
    Revision3NpcProfileEditEvaluationV1, Revision3NpcProfileEditOutcomeV1,
    Revision3NpcProfileEditPublicationStatusV1, Revision3NpcProfileEditRequestJsonErrorV1,
    Revision3NpcProfileEditRequestV1, Revision3NpcProfileEditRuntimeStatusV1, Revision3OriginRef,
    Revision3TypedRef, SchemaRevisionV3, Sha256Digest, WorkingHead, WorkingStoreFormat,
    LOGICAL_NPC_CLONE_GENERATOR_ID, LOGICAL_NPC_CLONE_GENERATOR_VERSION,
    MAX_REVISION3_NPC_PROFILE_EDIT_REQUEST_JSON_BYTES_V1,
};

const CURRENT_CATALOG_ID: &str = "g1r:npc:om_grd_asghan_263";
const DESIRED_CATALOG_ID: &str = "g1r:npc:om_stt_viper_302";

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

fn parent(
    target: &GameGenerationAnchor,
    seal_value: u8,
    selector_value: u8,
    runtime_class: &str,
) -> Revision2NpcParentClassInput {
    Revision2NpcParentClassInput {
        generation: target.clone(),
        source_seal: seal(seal_value, 40_000 + u64::from(seal_value)),
        catalog_layer: "base-game.g1r.scripts".to_owned(),
        canonical_selector: format!("Catalog_{}", format!("{selector_value:02x}").repeat(32)),
        runtime_class: runtime_class.to_owned(),
    }
}

fn current_selection(project: &ProjectRevision3) -> Revision3NpcCatalogSelectionV1 {
    Revision3NpcCatalogSelectionV1 {
        generation: project.target.clone(),
        catalog_id: CURRENT_CATALOG_ID.to_owned(),
        story_catalog_seal: seal(0x31, 5000),
        npc_catalog_seal: seal(0x32, 1_800_000),
        parent_character_definition: parent(
            &project.target,
            0x41,
            0x51,
            "UCharacterDefinition_Human_OM_GRD_Asghan_263",
        ),
        parent_ai_agent_config: parent(
            &project.target,
            0x42,
            0x52,
            "UAIAgentConfig_Human_OM_GRD_Asghan_263",
        ),
        parent_spawn_definition: parent(
            &project.target,
            0x43,
            0x53,
            "USpawnAIAgentDefinition_OM_GRD_Asghan_263",
        ),
    }
}

fn desired_selection(project: &ProjectRevision3) -> Revision3NpcCatalogSelectionV1 {
    Revision3NpcCatalogSelectionV1 {
        generation: project.target.clone(),
        catalog_id: DESIRED_CATALOG_ID.to_owned(),
        story_catalog_seal: seal(0x31, 5000),
        npc_catalog_seal: seal(0x32, 1_800_000),
        parent_character_definition: parent(
            &project.target,
            0x61,
            0x71,
            "UCharacterDefinition_Human_OM_STT_Viper_302",
        ),
        parent_ai_agent_config: parent(
            &project.target,
            0x62,
            0x72,
            "UAIAgentConfig_Human_OM_STT_Viper_302",
        ),
        parent_spawn_definition: parent(
            &project.target,
            0x63,
            0x73,
            "USpawnAIAgentDefinition_OM_STT_Viper_302",
        ),
    }
}

fn same_as_current_with_desired_id(project: &ProjectRevision3) -> Revision3NpcCatalogSelectionV1 {
    let current = current_selection(project);
    Revision3NpcCatalogSelectionV1 {
        catalog_id: DESIRED_CATALOG_ID.to_owned(),
        ..current
    }
}

fn name_context(project: &ProjectRevision3) -> Revision3NpcProfileCatalogContextV1 {
    Revision3NpcProfileCatalogContextV1 {
        current_selection: current_selection(project),
        desired_selection: current_selection(project),
    }
}

fn archetype_context(project: &ProjectRevision3) -> Revision3NpcProfileCatalogContextV1 {
    Revision3NpcProfileCatalogContextV1 {
        current_selection: current_selection(project),
        desired_selection: desired_selection(project),
    }
}

fn project() -> ProjectRevision3 {
    let project_id = project_id(0x11);
    let target = target(0x21);
    let npc_id = id(0x61);
    let module_id = id(0x62);
    let unrelated_id = id(0x70);
    let current = Revision3NpcCatalogSelectionV1 {
        generation: target.clone(),
        catalog_id: CURRENT_CATALOG_ID.to_owned(),
        story_catalog_seal: seal(0x31, 5000),
        npc_catalog_seal: seal(0x32, 1_800_000),
        parent_character_definition: parent(
            &target,
            0x41,
            0x51,
            "UCharacterDefinition_Human_OM_GRD_Asghan_263",
        ),
        parent_ai_agent_config: parent(
            &target,
            0x42,
            0x52,
            "UAIAgentConfig_Human_OM_GRD_Asghan_263",
        ),
        parent_spawn_definition: parent(
            &target,
            0x43,
            0x53,
            "USpawnAIAgentDefinition_OM_GRD_Asghan_263",
        ),
    };
    let npc = Revision3NpcDraft {
        generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
        generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
        input: Revision2NpcDraftInput {
            target: target.clone(),
            module_namespace: "GoreMods.Npcs.GateGuard7A1B2C3D4E".to_owned(),
            unique_name: "GORE_GATE_GUARD_7A1B2C3D4E".to_owned(),
            parent_character_definition: current.parent_character_definition,
            parent_ai_agent_config: current.parent_ai_agent_config,
            parent_spawn_definition: current.parent_spawn_definition,
        },
        script_module: Revision3TypedRef::new(
            project_id,
            module_id,
            Revision3EntityKind::ScriptModule,
        ),
        greetings: Vec::new(),
    };
    let module = npc
        .regenerate_script_module(Revision3TypedRef::new(
            project_id,
            npc_id,
            Revision3EntityKind::NpcDraft,
        ))
        .unwrap();
    let owner = module.owner.clone();
    let mut entities = BTreeMap::new();
    entities.insert(
        npc_id,
        Revision3Entity {
            id: npc_id,
            display_name: "Gate guard".to_owned(),
            origin: Revision3OriginRef::New {
                authored_runtime_id: npc.input.unique_name.clone(),
            },
            revision: 4,
            payload: Revision3EntityPayload::NpcDraft(npc),
        },
    );
    entities.insert(
        module_id,
        Revision3Entity {
            id: module_id,
            display_name: "Gate guard source".to_owned(),
            origin: Revision3OriginRef::Generated {
                generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
                generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
                owner,
            },
            revision: 9,
            payload: Revision3EntityPayload::ScriptModule(module),
        },
    );
    entities.insert(
        unrelated_id,
        Revision3Entity {
            id: unrelated_id,
            display_name: "Unrelated text".to_owned(),
            origin: Revision3OriginRef::New {
                authored_runtime_id: "GORE_UNRELATED_TEXT".to_owned(),
            },
            revision: 6,
            payload: Revision3EntityPayload::LocalizationEntry(Revision2LocalizationEntry {
                loc_id: "GORE_UNRELATED_TEXT".to_owned(),
                texts: BTreeMap::new(),
            }),
        },
    );
    let mut assets = BTreeMap::new();
    assets.insert(
        seal(0x91, 1234).sha256,
        AssetMeta {
            byte_len: 1234,
            media_type: "application/octet-stream".to_owned(),
        },
    );
    let project = ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id,
        revision: 7,
        meta: ProjectMeta {
            name: "NPC profile tests".to_owned(),
            version: "1.0.0".to_owned(),
            author: "tests".to_owned(),
        },
        target,
        authoring_locales: BTreeSet::new(),
        entities,
        asset_store: AssetStoreIndex { assets },
    };
    project.validate_closed_model().unwrap();
    project
}

fn request(
    project: &ProjectRevision3,
    basis_head: &WorkingHead,
    display_name: &str,
    parent_catalog_id: &str,
) -> Revision3NpcProfileEditRequestV1 {
    Revision3NpcProfileEditRequestV1 {
        expected_head: basis_head.clone(),
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        expected_target: project.target.clone(),
        expected_story_catalog_seal: seal(0x31, 5000),
        expected_npc_catalog_seal: seal(0x32, 1_800_000),
        npc_id: id(0x61),
        expected_npc_revision: project.entities[&id(0x61)].revision,
        script_module_id: id(0x62),
        expected_script_module_revision: project.entities[&id(0x62)].revision,
        expected_parent_catalog_id: CURRENT_CATALOG_ID.to_owned(),
        display_name: display_name.to_owned(),
        parent_catalog_id: parent_catalog_id.to_owned(),
    }
}

fn evaluate(
    project: &ProjectRevision3,
    request: &Revision3NpcProfileEditRequestV1,
    context: Revision3NpcProfileCatalogContextV1,
) -> Result<Revision3NpcProfileEditEvaluationV1, Revision3NpcProfileEditErrorV1> {
    apply_revision3_npc_profile_edit_transaction_v1(
        &request.expected_head,
        &project.to_canonical_json().unwrap(),
        &request.to_canonical_json().unwrap(),
        context,
    )
}

fn applied(value: Revision3NpcProfileEditEvaluationV1) -> Revision3NpcProfileEditOutcomeV1 {
    match value {
        Revision3NpcProfileEditEvaluationV1::Applied(outcome) => *outcome,
        Revision3NpcProfileEditEvaluationV1::Rejected(rejection) => {
            panic!("unexpected rejection: {}", rejection.conflict)
        }
    }
}

fn rejected(value: Revision3NpcProfileEditEvaluationV1) -> Revision3NpcProfileEditConflictV1 {
    match value {
        Revision3NpcProfileEditEvaluationV1::Rejected(rejection) => rejection.conflict,
        Revision3NpcProfileEditEvaluationV1::Applied(_) => panic!("unexpected candidate"),
    }
}

#[test]
fn name_only_changes_npc_shell_and_preserves_complete_module_assets_and_unrelated_entities() {
    let project = project();
    let request = request(
        &project,
        &head(0x71),
        "Castle gate guard",
        CURRENT_CATALOG_ID,
    );
    let outcome = applied(evaluate(&project, &request, name_context(&project)).unwrap());

    assert_eq!(outcome.project.revision, project.revision + 1);
    assert_eq!(outcome.npc_revision, 5);
    assert_eq!(outcome.script_module_revision, 9);
    assert!(outcome.name_changed);
    assert!(!outcome.archetype_changed);
    assert!(!outcome.module_regenerated);
    assert_eq!(
        outcome.project.entities[&id(0x62)],
        project.entities[&id(0x62)]
    );
    let before_npc = &project.entities[&id(0x61)];
    let after_npc = &outcome.project.entities[&id(0x61)];
    assert_eq!(after_npc.display_name, "Castle gate guard");
    assert_eq!(after_npc.revision, before_npc.revision + 1);
    assert_eq!(after_npc.origin, before_npc.origin);
    assert_eq!(after_npc.payload, before_npc.payload);
    assert_eq!(
        outcome.project.entities[&id(0x70)],
        project.entities[&id(0x70)]
    );
    assert_eq!(outcome.project.asset_store, project.asset_store);
    assert_eq!(
        outcome.build_status,
        Revision3NpcProfileEditBuildStatusV1::Blocked
    );
    assert_eq!(
        outcome.runtime_status,
        Revision3NpcProfileEditRuntimeStatusV1::RuntimeUnqualified
    );
    assert_eq!(
        outcome.catalog_authority,
        Revision3NpcProfileEditCatalogAuthorityV1::NotGranted
    );
    assert_eq!(
        outcome.collision_authority,
        Revision3NpcProfileEditCollisionAuthorityV1::NotGranted
    );
    assert_eq!(
        outcome.publication_status,
        Revision3NpcProfileEditPublicationStatusV1::NotSupported
    );
    assert_eq!(
        ProjectRevision3::from_json(&outcome.canonical_project_json).unwrap(),
        outcome.project
    );
}

#[test]
fn archetype_only_swaps_all_three_parents_and_regenerates_only_module_payload() {
    let project = project();
    let request = request(&project, &head(0x71), "Gate guard", DESIRED_CATALOG_ID);
    let desired = desired_selection(&project);
    let outcome = applied(evaluate(&project, &request, archetype_context(&project)).unwrap());

    assert!(!outcome.name_changed);
    assert!(outcome.archetype_changed);
    assert!(outcome.module_regenerated);
    assert_eq!(outcome.npc_revision, 5);
    assert_eq!(outcome.script_module_revision, 10);
    let before_npc_entity = &project.entities[&id(0x61)];
    let after_npc_entity = &outcome.project.entities[&id(0x61)];
    assert_eq!(
        after_npc_entity.display_name,
        before_npc_entity.display_name
    );
    assert_eq!(after_npc_entity.origin, before_npc_entity.origin);
    let Revision3EntityPayload::NpcDraft(before_npc) = &before_npc_entity.payload else {
        panic!("expected NPC")
    };
    let Revision3EntityPayload::NpcDraft(after_npc) = &after_npc_entity.payload else {
        panic!("expected NPC")
    };
    assert_eq!(
        after_npc.input.parent_character_definition,
        desired.parent_character_definition
    );
    assert_eq!(
        after_npc.input.parent_ai_agent_config,
        desired.parent_ai_agent_config
    );
    assert_eq!(
        after_npc.input.parent_spawn_definition,
        desired.parent_spawn_definition
    );
    assert_eq!(after_npc.input.unique_name, before_npc.input.unique_name);
    assert_eq!(
        after_npc.input.module_namespace,
        before_npc.input.module_namespace
    );
    assert_eq!(after_npc.input.target, before_npc.input.target);
    assert_eq!(after_npc.script_module, before_npc.script_module);
    assert_eq!(after_npc.generator_id, before_npc.generator_id);
    assert_eq!(after_npc.generator_version, before_npc.generator_version);

    let before_module_entity = &project.entities[&id(0x62)];
    let after_module_entity = &outcome.project.entities[&id(0x62)];
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
        panic!("expected module")
    };
    let Revision3EntityPayload::ScriptModule(after_module) = &after_module_entity.payload else {
        panic!("expected module")
    };
    assert_eq!(after_module.generator_id, before_module.generator_id);
    assert_eq!(
        after_module.generator_version,
        before_module.generator_version
    );
    assert_eq!(after_module.owner, before_module.owner);
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
    assert_ne!(after_module.source_sha256, before_module.source_sha256);
    assert_ne!(
        after_module.input_fingerprint,
        before_module.input_fingerprint
    );
    assert_eq!(
        outcome.project.entities[&id(0x70)],
        project.entities[&id(0x70)]
    );
    assert_eq!(outcome.project.asset_store, project.asset_store);
}

#[test]
fn combined_edit_is_deterministic_and_changes_each_revision_exactly_once() {
    let project = project();
    let request = request(
        &project,
        &head(0x71),
        "Viper-trained gate guard",
        DESIRED_CATALOG_ID,
    );
    let first = applied(evaluate(&project, &request, archetype_context(&project)).unwrap());
    let second = applied(evaluate(&project, &request, archetype_context(&project)).unwrap());
    assert_eq!(first, second);
    assert_eq!(first.project.revision, 8);
    assert_eq!(first.npc_revision, 5);
    assert_eq!(first.script_module_revision, 10);
    assert!(first.name_changed && first.archetype_changed && first.module_regenerated);
}

#[test]
fn equivalent_parent_triples_are_noop_or_name_only_even_under_a_different_catalog_id() {
    let project = project();
    let no_change = request(&project, &head(0x71), "Gate guard", DESIRED_CATALOG_ID);
    let equivalent_context = || Revision3NpcProfileCatalogContextV1 {
        current_selection: current_selection(&project),
        desired_selection: same_as_current_with_desired_id(&project),
    };
    assert_eq!(
        rejected(evaluate(&project, &no_change, equivalent_context()).unwrap()),
        Revision3NpcProfileEditConflictV1::NoChanges
    );

    let name = request(
        &project,
        &head(0x71),
        "Renamed equivalent guard",
        DESIRED_CATALOG_ID,
    );
    let outcome = applied(evaluate(&project, &name, equivalent_context()).unwrap());
    assert!(!outcome.archetype_changed);
    assert!(!outcome.module_regenerated);
    assert_eq!(
        outcome.project.entities[&id(0x62)],
        project.entities[&id(0x62)]
    );
}

#[test]
fn profile_name_and_archetype_edits_preserve_ordered_npc_greetings_and_shared_lines() {
    let mut project = project();
    let localization_id = id(0x80);
    let line_id = id(0x81);
    project.entities.insert(
        localization_id,
        Revision3Entity {
            id: localization_id,
            display_name: "Greeting localization".to_owned(),
            origin: Revision3OriginRef::New {
                authored_runtime_id: "GORE_PROFILE_GREETING_LOC".to_owned(),
            },
            revision: 2,
            payload: Revision3EntityPayload::LocalizationEntry(Revision2LocalizationEntry {
                loc_id: "GORE_PROFILE_GREETING".to_owned(),
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
                authored_runtime_id: "GORE_PROFILE_GREETING_LINE".to_owned(),
            },
            revision: 3,
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
    let greeting = Revision3NpcGreetingBindingV1 {
        line: Revision3TypedRef::new(project.project_id, line_id, Revision3EntityKind::DialogLine),
    };
    let Revision3EntityPayload::NpcDraft(npc) =
        &mut project.entities.get_mut(&id(0x61)).unwrap().payload
    else {
        unreachable!()
    };
    npc.greetings = vec![greeting.clone()];
    project.validate_closed_model().unwrap();
    let basis_line = project.entities[&line_id].clone();
    let basis_localization = project.entities[&localization_id].clone();

    let name_request = request(
        &project,
        &head(0x71),
        "Greeting castle guard",
        CURRENT_CATALOG_ID,
    );
    let named = applied(evaluate(&project, &name_request, name_context(&project)).unwrap());
    let Revision3EntityPayload::NpcDraft(named_npc) = &named.project.entities[&id(0x61)].payload
    else {
        unreachable!()
    };
    assert_eq!(named_npc.greetings, vec![greeting.clone()]);
    assert_eq!(named.project.entities[&line_id], basis_line);
    assert_eq!(named.project.entities[&localization_id], basis_localization);

    let archetype_request = request(&project, &head(0x71), "Gate guard", DESIRED_CATALOG_ID);
    let archetyped =
        applied(evaluate(&project, &archetype_request, archetype_context(&project)).unwrap());
    let Revision3EntityPayload::NpcDraft(archetyped_npc) =
        &archetyped.project.entities[&id(0x61)].payload
    else {
        unreachable!()
    };
    assert_eq!(archetyped_npc.greetings, vec![greeting]);
    assert_eq!(archetyped.project.entities[&line_id], basis_line);
    assert_eq!(
        archetyped.project.entities[&localization_id],
        basis_localization
    );
}

#[test]
fn request_wire_is_bounded_canonical_duplicate_safe_and_closed() {
    let project = project();
    let request = request(&project, &head(0x71), "Castle guard", CURRENT_CATALOG_ID);
    let canonical = request.to_canonical_json().unwrap();
    assert_eq!(
        Revision3NpcProfileEditRequestV1::from_json(&canonical).unwrap(),
        request
    );
    assert!(matches!(
        Revision3NpcProfileEditRequestV1::from_json(&format!(" {canonical}")),
        Err(Revision3NpcProfileEditRequestJsonErrorV1::NonCanonicalJson)
    ));
    let duplicate = canonical.replacen(
        "\"npc_id\":",
        &format!("\"npc_id\":\"{}\",\"npc_id\":", id(0x61)),
        1,
    );
    assert!(matches!(
        Revision3NpcProfileEditRequestV1::from_json(&duplicate),
        Err(Revision3NpcProfileEditRequestJsonErrorV1::InvalidJson(_))
    ));
    let unknown = canonical.replacen('{', "{\"authority\":false,", 1);
    assert!(matches!(
        Revision3NpcProfileEditRequestV1::from_json(&unknown),
        Err(Revision3NpcProfileEditRequestJsonErrorV1::InvalidJson(_))
    ));
    let oversized = "x".repeat(MAX_REVISION3_NPC_PROFILE_EDIT_REQUEST_JSON_BYTES_V1 + 1);
    assert!(matches!(
        Revision3NpcProfileEditRequestV1::from_json(&oversized),
        Err(Revision3NpcProfileEditRequestJsonErrorV1::InputTooLarge { .. })
    ));
}

#[test]
fn exact_head_project_target_entity_and_module_bindings_fail_closed() {
    let project = project();
    let basis_head = head(0x71);
    let baseline = request(&project, &basis_head, "Castle guard", CURRENT_CATALOG_ID);

    let mut stale = baseline.clone();
    stale.expected_head = head(0x72);
    assert_eq!(
        rejected(
            apply_revision3_npc_profile_edit_transaction_v1(
                &basis_head,
                &project.to_canonical_json().unwrap(),
                &stale.to_canonical_json().unwrap(),
                name_context(&project),
            )
            .unwrap()
        ),
        Revision3NpcProfileEditConflictV1::CurrentHeadMismatch
    );

    let cases = [
        {
            let mut value = baseline.clone();
            value.expected_project_id = project_id(0x99);
            value
        },
        {
            let mut value = baseline.clone();
            value.expected_revision -= 1;
            value
        },
        {
            let mut value = baseline.clone();
            value.expected_target = target(0x99);
            value
        },
        {
            let mut value = baseline.clone();
            value.expected_npc_revision -= 1;
            value
        },
        {
            let mut value = baseline.clone();
            value.expected_script_module_revision -= 1;
            value
        },
    ];
    for request in cases {
        assert!(matches!(
            rejected(evaluate(&project, &request, name_context(&project)).unwrap()),
            Revision3NpcProfileEditConflictV1::ProjectIdentityMismatch { .. }
                | Revision3NpcProfileEditConflictV1::ProjectRevisionConflict { .. }
                | Revision3NpcProfileEditConflictV1::ProjectTargetMismatch
                | Revision3NpcProfileEditConflictV1::NpcRevisionConflict { .. }
                | Revision3NpcProfileEditConflictV1::ScriptModuleRevisionConflict { .. }
        ));
    }

    let mut wrong_module = baseline;
    wrong_module.script_module_id = id(0x70);
    assert!(matches!(
        rejected(evaluate(&project, &wrong_module, name_context(&project)).unwrap()),
        Revision3NpcProfileEditConflictV1::NpcModuleBindingMismatch { .. }
    ));
}

#[test]
fn catalog_ids_seals_generation_and_stored_archetype_are_exactly_bound() {
    let project = project();
    let baseline = request(&project, &head(0x71), "Castle guard", DESIRED_CATALOG_ID);

    let mut wrong_current_id = archetype_context(&project);
    wrong_current_id.current_selection.catalog_id = "g1r:npc:wrong".to_owned();
    assert_eq!(
        rejected(evaluate(&project, &baseline, wrong_current_id).unwrap()),
        Revision3NpcProfileEditConflictV1::CurrentCatalogSelectionMismatch
    );
    let mut wrong_desired_id = archetype_context(&project);
    wrong_desired_id.desired_selection.catalog_id = "g1r:npc:wrong".to_owned();
    assert_eq!(
        rejected(evaluate(&project, &baseline, wrong_desired_id).unwrap()),
        Revision3NpcProfileEditConflictV1::DesiredCatalogSelectionMismatch
    );
    let mut wrong_story_seal = archetype_context(&project);
    wrong_story_seal.desired_selection.story_catalog_seal = seal(0x99, 5000);
    assert_eq!(
        rejected(evaluate(&project, &baseline, wrong_story_seal).unwrap()),
        Revision3NpcProfileEditConflictV1::StoryCatalogSealMismatch
    );
    let mut wrong_npc_seal = archetype_context(&project);
    wrong_npc_seal.current_selection.npc_catalog_seal = seal(0x99, 1_800_000);
    assert_eq!(
        rejected(evaluate(&project, &baseline, wrong_npc_seal).unwrap()),
        Revision3NpcProfileEditConflictV1::NpcCatalogSealMismatch
    );
    let mut wrong_generation = archetype_context(&project);
    wrong_generation.desired_selection.generation = target(0x99);
    assert_eq!(
        rejected(evaluate(&project, &baseline, wrong_generation).unwrap()),
        Revision3NpcProfileEditConflictV1::CatalogGenerationMismatch
    );
    let mut wrong_stored = archetype_context(&project);
    wrong_stored.current_selection.parent_spawn_definition =
        desired_selection(&project).parent_spawn_definition;
    assert_eq!(
        rejected(evaluate(&project, &baseline, wrong_stored).unwrap()),
        Revision3NpcProfileEditConflictV1::StoredArchetypeMismatch
    );

    let same_id = request(&project, &head(0x71), "Castle guard", CURRENT_CATALOG_ID);
    let mut contradictory_same_id = archetype_context(&project);
    contradictory_same_id.desired_selection.catalog_id = CURRENT_CATALOG_ID.to_owned();
    assert_eq!(
        rejected(evaluate(&project, &same_id, contradictory_same_id).unwrap()),
        Revision3NpcProfileEditConflictV1::InvalidCatalogContext
    );
}

#[test]
fn invalid_names_and_true_noop_return_no_candidate() {
    let project = project();
    for value in ["", "   ", "\u{2003}", " Bob ", "bad\nname"] {
        let request = request(&project, &head(0x71), value, CURRENT_CATALOG_ID);
        assert_eq!(
            rejected(evaluate(&project, &request, name_context(&project)).unwrap()),
            Revision3NpcProfileEditConflictV1::InvalidDisplayName
        );
    }
    let too_long = "x".repeat(257);
    let too_long_request = request(&project, &head(0x71), &too_long, CURRENT_CATALOG_ID);
    assert_eq!(
        rejected(evaluate(&project, &too_long_request, name_context(&project)).unwrap()),
        Revision3NpcProfileEditConflictV1::InvalidDisplayName
    );
    let no_op_request = request(&project, &head(0x71), "Gate guard", CURRENT_CATALOG_ID);
    assert_eq!(
        rejected(evaluate(&project, &no_op_request, name_context(&project)).unwrap()),
        Revision3NpcProfileEditConflictV1::NoChanges
    );
}

#[test]
fn revision_overflow_is_conditional_for_the_unchanged_module() {
    let mut module_max = project();
    module_max.entities.get_mut(&id(0x62)).unwrap().revision = u64::MAX;
    module_max.validate_closed_model().unwrap();
    let name = request(
        &module_max,
        &head(0x71),
        "Name survives module max",
        CURRENT_CATALOG_ID,
    );
    let name_outcome = applied(evaluate(&module_max, &name, name_context(&module_max)).unwrap());
    assert_eq!(name_outcome.script_module_revision, u64::MAX);
    assert_eq!(
        name_outcome.project.entities[&id(0x62)],
        module_max.entities[&id(0x62)]
    );

    let archetype = request(&module_max, &head(0x71), "Gate guard", DESIRED_CATALOG_ID);
    assert!(matches!(
        rejected(evaluate(&module_max, &archetype, archetype_context(&module_max)).unwrap()),
        Revision3NpcProfileEditConflictV1::ScriptModuleRevisionOverflow { .. }
    ));

    let mut npc_max = project();
    npc_max.entities.get_mut(&id(0x61)).unwrap().revision = u64::MAX;
    let npc_overflow_request = request(&npc_max, &head(0x71), "Castle guard", CURRENT_CATALOG_ID);
    assert!(matches!(
        rejected(evaluate(&npc_max, &npc_overflow_request, name_context(&npc_max)).unwrap()),
        Revision3NpcProfileEditConflictV1::NpcRevisionOverflow { .. }
    ));

    let mut project_max = project();
    project_max.revision = u64::MAX;
    let project_overflow_request = request(
        &project_max,
        &head(0x71),
        "Castle guard",
        CURRENT_CATALOG_ID,
    );
    assert_eq!(
        rejected(
            evaluate(
                &project_max,
                &project_overflow_request,
                name_context(&project_max),
            )
            .unwrap()
        ),
        Revision3NpcProfileEditConflictV1::ProjectRevisionOverflow
    );
}

#[test]
fn corrupted_generated_source_and_ownership_are_rejected_as_invalid_exact_projects() {
    for corrupt in [
        |project: &mut ProjectRevision3| {
            let Revision3EntityPayload::ScriptModule(module) =
                &mut project.entities.get_mut(&id(0x62)).unwrap().payload
            else {
                unreachable!()
            };
            module.source.push_str("// drift");
        },
        |project: &mut ProjectRevision3| {
            let Revision3EntityPayload::ScriptModule(module) =
                &mut project.entities.get_mut(&id(0x62)).unwrap().payload
            else {
                unreachable!()
            };
            module.owner.id = id(0x70);
        },
    ] {
        let mut corrupted = project();
        corrupt(&mut corrupted);
        let request = request(&corrupted, &head(0x71), "Castle guard", CURRENT_CATALOG_ID);
        let raw = serde_json::to_string(&corrupted).unwrap();
        assert!(matches!(
            apply_revision3_npc_profile_edit_transaction_v1(
                &request.expected_head,
                &raw,
                &request.to_canonical_json().unwrap(),
                name_context(&corrupted),
            ),
            Err(Revision3NpcProfileEditErrorV1::InvalidProject(_))
        ));
    }
}
