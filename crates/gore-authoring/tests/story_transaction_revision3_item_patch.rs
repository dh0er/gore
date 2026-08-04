use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::model_revision3::{Entity, EntityPayload, OriginRef};
use gore_authoring::{
    apply_revision3_item_patch_transaction_v1, build_revision3_content_index_v1, AssetStoreIndex,
    ContentSeal, EntityId, FormatV2, GameGenerationAnchor, ItemFiniteFloatErrorV1,
    ItemFiniteFloatV1, ItemPatchV1, ItemScalarTypeV1, ItemScalarValueV1, ProjectId, ProjectMeta,
    ProjectRevision3, ProjectRevision3ValidationError, Revision3ContentEntitySummaryV1,
    Revision3EntityKind, Revision3ItemPatchBuildStatusV1, Revision3ItemPatchChangeV1,
    Revision3ItemPatchConflictV1, Revision3ItemPatchEvaluationV1, Revision3ItemPatchMutationV1,
    Revision3ItemPatchPublicationStatusV1, Revision3ItemPatchRequestJsonErrorV1,
    Revision3ItemPatchRequestV1, Revision3ItemPatchRuntimeStatusV1, SchemaRevisionV3, Sha256Digest,
    WorkingHead, WorkingStoreFormat, MAX_REVISION3_ITEM_FIELD_NAME_BYTES_V1,
    MAX_REVISION3_ITEM_PATCH_FIELDS_V1, MAX_REVISION3_ITEM_PATCH_REQUEST_JSON_BYTES_V1,
    MAX_REVISION3_ITEM_STRING_BYTES_V1,
};

const ITEM_ID: u8 = 0x41;
const ITEM_CLASS: &str = "ItFo_Apple";
const CATALOG_LAYER: &str = "base-game.items.g1r.v1";

fn project_id(tag: u8) -> ProjectId {
    ProjectId::from_bytes([tag; 16])
}

fn id(tag: u8) -> EntityId {
    EntityId::from_bytes([tag; 16])
}

fn seal(tag: u8, byte_len: u64) -> ContentSeal {
    ContentSeal {
        byte_len,
        sha256: Sha256Digest::from_bytes([tag; 32]),
    }
}

fn target(tag: u8) -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: seal(tag, 171_698_176),
    }
}

fn head(tag: u8) -> WorkingHead {
    WorkingHead {
        store_format: WorkingStoreFormat,
        snapshot: seal(tag, 4096),
    }
}

fn empty_project() -> ProjectRevision3 {
    ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id: project_id(0x10),
        revision: 7,
        meta: ProjectMeta {
            name: "Managed items".to_owned(),
            version: "0.1.0".to_owned(),
            author: "tests".to_owned(),
        },
        target: target(0x20),
        authoring_locales: BTreeSet::new(),
        entities: BTreeMap::new(),
        asset_store: AssetStoreIndex::default(),
    }
}

fn fields(value: i64) -> BTreeMap<String, ItemScalarValueV1> {
    BTreeMap::from([
        ("m_Enabled".to_owned(), ItemScalarValueV1::Boolean(true)),
        (
            "m_Kind".to_owned(),
            ItemScalarValueV1::Enum {
                enum_type: "EItem::Kind".to_owned(),
                backing: 3,
            },
        ),
        ("m_MaxStack".to_owned(), ItemScalarValueV1::Integer(10)),
        (
            "m_Name".to_owned(),
            ItemScalarValueV1::String("Apple".to_owned()),
        ),
        ("m_Value".to_owned(), ItemScalarValueV1::Integer(value)),
        (
            "m_Weight".to_owned(),
            ItemScalarValueV1::Float(ItemFiniteFloatV1::new(0.25).unwrap()),
        ),
    ])
}

fn vanilla_origin(project: &ProjectRevision3, class: &str, source_tag: u8) -> OriginRef {
    OriginRef::Vanilla {
        generation: project.target.clone(),
        catalog_layer: CATALOG_LAYER.to_owned(),
        canonical_selector: class.to_owned(),
        source_seal: seal(source_tag, 1_048_576),
    }
}

fn item_entity(project: &ProjectRevision3, class: &str, source_tag: u8) -> Entity {
    Entity {
        id: id(ITEM_ID),
        display_name: "Apple".to_owned(),
        origin: vanilla_origin(project, class, source_tag),
        revision: 2,
        payload: EntityPayload::ItemPatch(ItemPatchV1 {
            vanilla_class: class.to_owned(),
            fields: fields(5),
        }),
    }
}

fn upsert_request(
    project: &ProjectRevision3,
    expected_entity_revision: Option<u64>,
    replacement_fields: BTreeMap<String, ItemScalarValueV1>,
) -> Revision3ItemPatchRequestV1 {
    Revision3ItemPatchRequestV1 {
        expected_head: head(0x30),
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        expected_target: project.target.clone(),
        mutation: Revision3ItemPatchMutationV1::Upsert {
            entity_id: id(ITEM_ID),
            expected_entity_revision,
            display_name: "Apple".to_owned(),
            catalog_layer: CATALOG_LAYER.to_owned(),
            vanilla_class: ITEM_CLASS.to_owned(),
            source_seal: seal(0x40, 1_048_576),
            fields: replacement_fields,
        },
    }
}

fn evaluate(
    project: &ProjectRevision3,
    request: &Revision3ItemPatchRequestV1,
) -> Revision3ItemPatchEvaluationV1 {
    apply_revision3_item_patch_transaction_v1(
        &head(0x30),
        &project.to_canonical_json().unwrap(),
        &request.to_canonical_json().unwrap(),
    )
    .unwrap()
}

fn applied(
    evaluation: Revision3ItemPatchEvaluationV1,
) -> gore_authoring::Revision3ItemPatchOutcomeV1 {
    match evaluation {
        Revision3ItemPatchEvaluationV1::Applied(outcome) => *outcome,
        Revision3ItemPatchEvaluationV1::Rejected(rejection) => {
            panic!("unexpected rejection: {:?}", rejection.conflict)
        }
    }
}

fn rejected(evaluation: Revision3ItemPatchEvaluationV1) -> Revision3ItemPatchConflictV1 {
    match evaluation {
        Revision3ItemPatchEvaluationV1::Rejected(rejection) => rejection.conflict,
        Revision3ItemPatchEvaluationV1::Applied(_) => panic!("unexpected application"),
    }
}

#[test]
fn item_patch_model_roundtrips_canonically_and_projects_typed_content_facts() {
    let mut project = empty_project();
    let entity = item_entity(&project, ITEM_CLASS, 0x40);
    project.entities.insert(entity.id, entity);

    let canonical = project.to_canonical_json().unwrap();
    assert_eq!(ProjectRevision3::from_json(&canonical).unwrap(), project);
    assert!(canonical.find("m_Enabled").unwrap() < canonical.find("m_Value").unwrap());
    assert!(canonical.contains("\"kind\":\"item_patch\""));
    assert!(canonical.contains("\"type\":\"float\",\"data\":0.25"));
    assert!(canonical
        .contains("\"type\":\"enum\",\"data\":{\"enum_type\":\"EItem::Kind\",\"backing\":3}"));

    let index = build_revision3_content_index_v1(&project).unwrap();
    assert_eq!(
        index.entity_counts.get(&Revision3EntityKind::ItemPatch),
        Some(&1)
    );
    let summary = &index.entities[0].summary;
    let Revision3ContentEntitySummaryV1::ItemPatch {
        vanilla_class,
        field_count,
        field_types,
        fields: projected_fields,
    } = summary
    else {
        panic!("expected item-patch summary")
    };
    assert_eq!(vanilla_class, ITEM_CLASS);
    assert_eq!(*field_count, 6);
    assert_eq!(field_types["m_Weight"], ItemScalarTypeV1::Float);
    assert_eq!(field_types["m_Kind"], ItemScalarTypeV1::Enum);
    assert_eq!(projected_fields, &fields(5));
    assert!(index.entities[0].references.is_empty());
    assert!(index.entities[0].asset_references.is_empty());
}

#[test]
fn item_patch_model_rejects_wrong_provenance_names_and_closed_budget_violations() {
    let mut project = empty_project();
    let base = item_entity(&project, ITEM_CLASS, 0x40);
    project.entities.insert(base.id, base);

    let mut wrong_generation = project.clone();
    let OriginRef::Vanilla { generation, .. } = &mut wrong_generation
        .entities
        .get_mut(&id(ITEM_ID))
        .unwrap()
        .origin
    else {
        unreachable!()
    };
    *generation = target(0x99);
    assert!(matches!(
        wrong_generation.validate_closed_model(),
        Err(ProjectRevision3ValidationError::InvalidItemPatch { .. })
    ));

    let mut wrong_origin = project.clone();
    wrong_origin.entities.get_mut(&id(ITEM_ID)).unwrap().origin = OriginRef::New {
        authored_runtime_id: ITEM_CLASS.to_owned(),
    };
    assert!(matches!(
        wrong_origin.validate_closed_model(),
        Err(ProjectRevision3ValidationError::InvalidItemPatch { .. })
    ));

    let mut selector_drift = project.clone();
    let OriginRef::Vanilla {
        canonical_selector, ..
    } = &mut selector_drift
        .entities
        .get_mut(&id(ITEM_ID))
        .unwrap()
        .origin
    else {
        unreachable!()
    };
    *canonical_selector = "ItFo_Bread".to_owned();
    assert!(matches!(
        selector_drift.validate_closed_model(),
        Err(ProjectRevision3ValidationError::InvalidItemPatch { .. })
    ));

    let mut zero_source = project.clone();
    let OriginRef::Vanilla { source_seal, .. } =
        &mut zero_source.entities.get_mut(&id(ITEM_ID)).unwrap().origin
    else {
        unreachable!()
    };
    source_seal.byte_len = 0;
    assert!(matches!(
        zero_source.validate_closed_model(),
        Err(ProjectRevision3ValidationError::InvalidItemPatch { .. })
    ));

    let mut bad_catalog = project.clone();
    let OriginRef::Vanilla { catalog_layer, .. } =
        &mut bad_catalog.entities.get_mut(&id(ITEM_ID)).unwrap().origin
    else {
        unreachable!()
    };
    *catalog_layer = " bad catalog ".to_owned();
    assert!(matches!(
        bad_catalog.validate_closed_model(),
        Err(ProjectRevision3ValidationError::InvalidItemPatch { .. })
    ));

    let mut bad_class = project.clone();
    let bad_entity = bad_class.entities.get_mut(&id(ITEM_ID)).unwrap();
    let OriginRef::Vanilla {
        canonical_selector, ..
    } = &mut bad_entity.origin
    else {
        unreachable!()
    };
    *canonical_selector = "ItFo-Apple".to_owned();
    let EntityPayload::ItemPatch(patch) = &mut bad_entity.payload else {
        unreachable!()
    };
    patch.vanilla_class = "ItFo-Apple".to_owned();
    assert!(matches!(
        bad_class.validate_closed_model(),
        Err(ProjectRevision3ValidationError::InvalidItemPatch { .. })
    ));

    let invalid_maps = [
        BTreeMap::new(),
        BTreeMap::from([(
            format!("m_{}", "x".repeat(MAX_REVISION3_ITEM_FIELD_NAME_BYTES_V1)),
            ItemScalarValueV1::Integer(1),
        )]),
        BTreeMap::from([(
            "m_Text".to_owned(),
            ItemScalarValueV1::String("x".repeat(MAX_REVISION3_ITEM_STRING_BYTES_V1 + 1)),
        )]),
        BTreeMap::from([(
            "m_Kind".to_owned(),
            ItemScalarValueV1::Enum {
                enum_type: "Bad::".to_owned(),
                backing: 1,
            },
        )]),
    ];
    for invalid_fields in invalid_maps {
        let mut candidate = project.clone();
        let EntityPayload::ItemPatch(patch) =
            &mut candidate.entities.get_mut(&id(ITEM_ID)).unwrap().payload
        else {
            unreachable!()
        };
        patch.fields = invalid_fields;
        assert!(matches!(
            candidate.validate_closed_model(),
            Err(ProjectRevision3ValidationError::InvalidItemPatch { .. })
        ));
    }

    let mut too_many = project.clone();
    let EntityPayload::ItemPatch(patch) =
        &mut too_many.entities.get_mut(&id(ITEM_ID)).unwrap().payload
    else {
        unreachable!()
    };
    patch.fields = (0..=MAX_REVISION3_ITEM_PATCH_FIELDS_V1)
        .map(|ordinal| (format!("m_Field_{ordinal}"), ItemScalarValueV1::Integer(1)))
        .collect();
    assert!(matches!(
        too_many.validate_closed_model(),
        Err(ProjectRevision3ValidationError::InvalidItemPatch { .. })
    ));

    let mut duplicate = project.clone();
    let mut second = duplicate.entities[&id(ITEM_ID)].clone();
    second.id = id(ITEM_ID + 1);
    let OriginRef::Vanilla {
        catalog_layer,
        source_seal,
        ..
    } = &mut second.origin
    else {
        unreachable!()
    };
    *catalog_layer = "alternate.items.catalog".to_owned();
    *source_seal = seal(0x77, 2_048);
    duplicate.entities.insert(second.id, second);
    assert!(matches!(
        duplicate.validate_closed_model(),
        Err(ProjectRevision3ValidationError::DuplicateItemPatchTarget { .. })
    ));
}

#[test]
fn finite_item_float_rejects_nonfinite_and_normalizes_negative_zero() {
    assert_eq!(
        ItemFiniteFloatV1::new(f64::NAN),
        Err(ItemFiniteFloatErrorV1::NotFinite)
    );
    assert_eq!(
        ItemFiniteFloatV1::new(f64::INFINITY),
        Err(ItemFiniteFloatErrorV1::NotFinite)
    );
    assert_eq!(
        ItemFiniteFloatV1::new(-0.0).unwrap(),
        ItemFiniteFloatV1::new(0.0).unwrap()
    );
    assert_eq!(
        ItemFiniteFloatV1::new(-0.0).unwrap().get().to_bits(),
        0.0f64.to_bits()
    );
}

#[test]
fn exact_basis_transaction_is_deterministic_across_create_update_noop_and_remove() {
    let basis = empty_project();
    let create_request = upsert_request(&basis, None, fields(5));
    let created_a = applied(evaluate(&basis, &create_request));
    let created_b = applied(evaluate(&basis, &create_request));
    assert_eq!(
        created_a.canonical_project_json,
        created_b.canonical_project_json
    );
    assert_eq!(created_a.change, Revision3ItemPatchChangeV1::Created);
    assert_eq!(created_a.project.revision, 8);
    assert_eq!(created_a.entity_revision, Some(0));
    assert_eq!(
        created_a.build_status,
        Revision3ItemPatchBuildStatusV1::Blocked
    );
    assert_eq!(
        created_a.runtime_status,
        Revision3ItemPatchRuntimeStatusV1::RuntimeUnqualified
    );
    assert_eq!(
        created_a.publication_status,
        Revision3ItemPatchPublicationStatusV1::NotSupported
    );
    assert_eq!(
        ProjectRevision3::from_json(&created_a.canonical_project_json).unwrap(),
        created_a.project
    );

    let no_op_request = upsert_request(&created_a.project, Some(0), fields(5));
    assert_eq!(
        rejected(evaluate(&created_a.project, &no_op_request)),
        Revision3ItemPatchConflictV1::NoChanges
    );
    assert_eq!(created_a.project.revision, 8);
    assert_eq!(created_a.project.entities[&id(ITEM_ID)].revision, 0);

    let update_request = upsert_request(&created_a.project, Some(0), fields(42));
    let updated = applied(evaluate(&created_a.project, &update_request));
    assert_eq!(updated.change, Revision3ItemPatchChangeV1::Updated);
    assert_eq!(updated.project.revision, 9);
    assert_eq!(updated.entity_revision, Some(1));
    let EntityPayload::ItemPatch(updated_patch) = &updated.project.entities[&id(ITEM_ID)].payload
    else {
        unreachable!()
    };
    assert_eq!(
        updated_patch.fields["m_Value"],
        ItemScalarValueV1::Integer(42)
    );

    let remove_request = Revision3ItemPatchRequestV1 {
        expected_head: head(0x30),
        expected_project_id: updated.project.project_id,
        expected_revision: updated.project.revision,
        expected_target: updated.project.target.clone(),
        mutation: Revision3ItemPatchMutationV1::Remove {
            entity_id: id(ITEM_ID),
            expected_entity_revision: 1,
            expected_catalog_layer: CATALOG_LAYER.to_owned(),
            expected_vanilla_class: ITEM_CLASS.to_owned(),
            expected_source_seal: seal(0x40, 1_048_576),
        },
    };
    let removed = applied(evaluate(&updated.project, &remove_request));
    assert_eq!(removed.change, Revision3ItemPatchChangeV1::Removed);
    assert_eq!(removed.project.revision, 10);
    assert_eq!(removed.entity_revision, None);
    assert!(!removed.project.entities.contains_key(&id(ITEM_ID)));
}

#[test]
fn transaction_rejects_stale_basis_identity_provenance_and_duplicate_targets() {
    let basis = empty_project();
    let request = upsert_request(&basis, None, fields(5));

    assert_eq!(
        rejected(
            apply_revision3_item_patch_transaction_v1(
                &head(0x31),
                &basis.to_canonical_json().unwrap(),
                &request.to_canonical_json().unwrap(),
            )
            .unwrap(),
        ),
        Revision3ItemPatchConflictV1::CurrentHeadMismatch
    );

    let mut stale_revision = request.clone();
    stale_revision.expected_revision -= 1;
    assert!(matches!(
        rejected(evaluate(&basis, &stale_revision)),
        Revision3ItemPatchConflictV1::ProjectRevisionConflict { .. }
    ));

    let mut wrong_target = request.clone();
    wrong_target.expected_target = target(0x99);
    assert_eq!(
        rejected(evaluate(&basis, &wrong_target)),
        Revision3ItemPatchConflictV1::ProjectTargetMismatch
    );

    let created = applied(evaluate(&basis, &request));
    let stale_entity = upsert_request(&created.project, Some(99), fields(42));
    assert!(matches!(
        rejected(evaluate(&created.project, &stale_entity)),
        Revision3ItemPatchConflictV1::EntityRevisionConflict { .. }
    ));

    let mut provenance_drift = upsert_request(&created.project, Some(0), fields(42));
    let Revision3ItemPatchMutationV1::Upsert { source_seal, .. } = &mut provenance_drift.mutation
    else {
        unreachable!()
    };
    *source_seal = seal(0x44, 1_048_576);
    assert_eq!(
        rejected(evaluate(&created.project, &provenance_drift)),
        Revision3ItemPatchConflictV1::ProvenanceConflict {
            entity: id(ITEM_ID)
        }
    );

    let duplicate_project = created.project.clone();
    let mut duplicate_request = upsert_request(&duplicate_project, None, fields(7));
    let Revision3ItemPatchMutationV1::Upsert { entity_id, .. } = &mut duplicate_request.mutation
    else {
        unreachable!()
    };
    *entity_id = id(ITEM_ID + 1);
    let Revision3ItemPatchMutationV1::Upsert {
        catalog_layer,
        source_seal,
        ..
    } = &mut duplicate_request.mutation
    else {
        unreachable!()
    };
    *catalog_layer = "alternate.items.catalog".to_owned();
    *source_seal = seal(0x78, 4_096);
    assert_eq!(
        rejected(evaluate(&duplicate_project, &duplicate_request)),
        Revision3ItemPatchConflictV1::DuplicateVanillaTarget
    );
}

#[test]
fn request_parser_rejects_arbitrary_values_duplicate_keys_noncanonical_and_oversize() {
    let project = empty_project();
    let request = upsert_request(&project, None, fields(5));
    let canonical = request.to_canonical_json().unwrap();
    assert_eq!(
        Revision3ItemPatchRequestV1::from_json(&canonical).unwrap(),
        request
    );
    assert!(matches!(
        Revision3ItemPatchRequestV1::from_json(&format!(" {canonical}")),
        Err(Revision3ItemPatchRequestJsonErrorV1::NonCanonicalJson)
    ));

    let duplicate = canonical.replacen(
        "\"expected_revision\":7",
        "\"expected_revision\":7,\"expected_revision\":7",
        1,
    );
    assert!(matches!(
        Revision3ItemPatchRequestV1::from_json(&duplicate),
        Err(Revision3ItemPatchRequestJsonErrorV1::InvalidJson(_))
    ));

    let mut arbitrary = serde_json::to_value(&request).unwrap();
    arbitrary["mutation"]["fields"]["m_Value"] = serde_json::json!({"anything": [1, 2, 3]});
    let arbitrary = serde_json::to_string(&arbitrary).unwrap();
    assert!(matches!(
        Revision3ItemPatchRequestV1::from_json(&arbitrary),
        Err(Revision3ItemPatchRequestJsonErrorV1::InvalidJson(_))
    ));

    assert!(matches!(
        Revision3ItemPatchRequestV1::from_json(
            &"x".repeat(MAX_REVISION3_ITEM_PATCH_REQUEST_JSON_BYTES_V1 + 1)
        ),
        Err(Revision3ItemPatchRequestJsonErrorV1::InputTooLarge { .. })
    ));
}
