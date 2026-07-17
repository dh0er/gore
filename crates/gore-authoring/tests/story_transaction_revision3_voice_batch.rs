use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::model_revision3::{
    DialogLine, Entity, EntityKind, EntityPayload, LocalizationEntry, OriginRef, SchemaRevisionV3,
    TypedRef, VoiceTakeStatus,
};
use gore_authoring::{
    apply_revision3_voice_take_batch_transaction_v1, AssetRef, AssetStoreIndex, ContentSeal,
    EntityId, FormatV2, GameGenerationAnchor, ImportedOgg, LocaleCode, OggCodec, OggMetadata,
    ProjectId, ProjectMeta, ProjectRevision3, Revision3VoiceTakeBatchConflictV1,
    Revision3VoiceTakeBatchEvaluationV1, Revision3VoiceTakeStageRequestV1, Sha256Digest,
    WorkingHead, WorkingStoreFormat,
};

fn id(tag: u8) -> EntityId {
    EntityId::from_bytes([tag; 16])
}

fn digest(tag: u8) -> Sha256Digest {
    Sha256Digest::from_bytes([tag; 32])
}

fn seal(tag: u8, byte_len: u64) -> ContentSeal {
    ContentSeal {
        byte_len,
        sha256: digest(tag),
    }
}

fn project_id() -> ProjectId {
    ProjectId::from_bytes([0x10; 16])
}

fn target() -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: seal(0x21, 171_698_176),
    }
}

fn head() -> WorkingHead {
    WorkingHead {
        store_format: WorkingStoreFormat,
        snapshot: seal(0x31, 4096),
    }
}

fn locale(value: &str) -> LocaleCode {
    value.parse().unwrap()
}

fn origin(tag: u8) -> OriginRef {
    OriginRef::Imported {
        importer: "tests".to_owned(),
        source_seal: seal(tag, 10),
        external_identity: None,
    }
}

fn basis() -> ProjectRevision3 {
    let mut entities = BTreeMap::new();
    for (localization_tag, line_tag, loc_id) in [
        (2, 3, "GRD_263_ASGHAN_OPEN_INFO_06_02"),
        (6, 7, "VLK_574_VIPER_HELLO_01_01"),
    ] {
        entities.insert(
            id(localization_tag),
            Entity {
                id: id(localization_tag),
                display_name: format!("Localization {localization_tag}"),
                origin: origin(localization_tag),
                revision: 4,
                payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                    loc_id: loc_id.to_owned(),
                    texts: BTreeMap::new(),
                }),
            },
        );
        entities.insert(
            id(line_tag),
            Entity {
                id: id(line_tag),
                display_name: format!("Line {line_tag}"),
                origin: origin(line_tag),
                revision: 2,
                payload: EntityPayload::DialogLine(DialogLine {
                    localization: TypedRef::new(
                        project_id(),
                        id(localization_tag),
                        EntityKind::LocalizationEntry,
                    ),
                    speaker_hint: None,
                    voice_slots: BTreeMap::new(),
                }),
            },
        );
    }
    ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id: project_id(),
        revision: 7,
        meta: ProjectMeta {
            name: "Voice batch".into(),
            version: "1.0.0".into(),
            author: "tests".into(),
        },
        target: target(),
        authoring_locales: BTreeSet::new(),
        entities,
        asset_store: AssetStoreIndex::default(),
    }
}

fn request(line_tag: u8, slot_tag: u8, take_tag: u8, locale_name: &str) -> String {
    let logical_name = match line_tag {
        3 => "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg",
        7 => "VLK_574_VIPER_HELLO_01_01.ogg",
        _ => unreachable!(),
    };
    Revision3VoiceTakeStageRequestV1 {
        expected_head: head(),
        expected_project_id: project_id(),
        expected_revision: 7,
        expected_target: target(),
        line_id: id(line_tag),
        slot_id: id(slot_tag),
        take_id: id(take_tag),
        locale: locale(locale_name),
        text: None,
        take_display_name: format!("Take {take_tag}"),
        logical_name: logical_name.to_owned(),
        status: VoiceTakeStatus::Recorded,
        select_take: false,
    }
    .to_canonical_json()
    .unwrap()
}

fn imported(tag: u8, logical_name: &str) -> ImportedOgg {
    ImportedOgg {
        asset: AssetRef {
            sha256: digest(tag),
            byte_len: 8192,
            logical_name: logical_name.to_owned(),
        },
        ogg: OggMetadata {
            codec: OggCodec::Vorbis,
            channels: 1,
            sample_rate: 48_000,
            pages: 3,
            logical_streams: 1,
        },
        deduplicated: false,
    }
}

#[test]
fn applies_two_items_as_one_project_revision_transaction() {
    let basis = basis();
    let requests = vec![request(3, 4, 5, "de"), request(7, 8, 9, "de")];
    let receipts = vec![
        imported(0x41, "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg"),
        imported(0x42, "VLK_574_VIPER_HELLO_01_01.ogg"),
    ];
    let Revision3VoiceTakeBatchEvaluationV1::Applied(outcome) =
        apply_revision3_voice_take_batch_transaction_v1(
            &head(),
            &basis.to_canonical_json().unwrap(),
            &requests,
            receipts,
        )
        .unwrap()
    else {
        panic!("expected atomic batch application")
    };

    assert_eq!(outcome.project.revision, 8);
    assert_eq!(outcome.items.len(), 2);
    assert!(outcome.items.iter().all(|item| item.slot_created));
    assert!(outcome.project.entities.contains_key(&id(4)));
    assert!(outcome.project.entities.contains_key(&id(5)));
    assert!(outcome.project.entities.contains_key(&id(8)));
    assert!(outcome.project.entities.contains_key(&id(9)));
    assert_eq!(outcome.project.entities[&id(3)].revision, 3);
    assert_eq!(outcome.project.entities[&id(7)].revision, 3);
    assert_eq!(
        ProjectRevision3::from_json(&outcome.canonical_project_json).unwrap(),
        outcome.project
    );
}

#[test]
fn rejects_the_whole_batch_when_any_item_conflicts() {
    let basis = basis();
    let mut second: Revision3VoiceTakeStageRequestV1 =
        serde_json::from_str(&request(7, 8, 9, "de")).unwrap();
    second.take_id = id(3); // Collides with an existing DialogLine entity.
    let requests = vec![request(3, 4, 5, "de"), second.to_canonical_json().unwrap()];
    let receipts = vec![
        imported(0x41, "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg"),
        imported(0x42, "VLK_574_VIPER_HELLO_01_01.ogg"),
    ];
    let Revision3VoiceTakeBatchEvaluationV1::Rejected(rejection) =
        apply_revision3_voice_take_batch_transaction_v1(
            &head(),
            &basis.to_canonical_json().unwrap(),
            &requests,
            receipts,
        )
        .unwrap()
    else {
        panic!("expected all-or-nothing rejection")
    };
    assert!(matches!(
        rejection.conflict,
        Revision3VoiceTakeBatchConflictV1::Item { item_index: 1, .. }
    ));
    assert_eq!(basis.revision, 7);
    assert!(!basis.entities.contains_key(&id(4)));
}

#[test]
fn enforces_one_locale_and_one_item_per_line() {
    let basis_json = basis().to_canonical_json().unwrap();
    let receipt = imported(0x41, "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg");

    let mixed = apply_revision3_voice_take_batch_transaction_v1(
        &head(),
        &basis_json,
        &[request(3, 4, 5, "de"), request(7, 8, 9, "en")],
        vec![
            receipt.clone(),
            imported(0x42, "VLK_574_VIPER_HELLO_01_01.ogg"),
        ],
    )
    .unwrap();
    assert!(matches!(
        mixed,
        Revision3VoiceTakeBatchEvaluationV1::Rejected(ref rejection)
            if matches!(rejection.conflict, Revision3VoiceTakeBatchConflictV1::MixedLocale { item_index: 1 })
    ));

    let duplicate = apply_revision3_voice_take_batch_transaction_v1(
        &head(),
        &basis_json,
        &[request(3, 4, 5, "de"), request(3, 10, 11, "de")],
        vec![receipt.clone(), receipt],
    )
    .unwrap();
    assert!(matches!(
        duplicate,
        Revision3VoiceTakeBatchEvaluationV1::Rejected(ref rejection)
            if matches!(rejection.conflict, Revision3VoiceTakeBatchConflictV1::DuplicateLine { item_index: 1 })
    ));

    let mut shared_slot: Revision3VoiceTakeStageRequestV1 =
        serde_json::from_str(&request(7, 8, 9, "de")).unwrap();
    shared_slot.slot_id = id(4);
    let duplicate_slot = apply_revision3_voice_take_batch_transaction_v1(
        &head(),
        &basis_json,
        &[
            request(3, 4, 5, "de"),
            shared_slot.to_canonical_json().unwrap(),
        ],
        vec![
            imported(0x41, "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg"),
            imported(0x42, "VLK_574_VIPER_HELLO_01_01.ogg"),
        ],
    )
    .unwrap();
    assert!(matches!(
        duplicate_slot,
        Revision3VoiceTakeBatchEvaluationV1::Rejected(ref rejection)
            if matches!(rejection.conflict, Revision3VoiceTakeBatchConflictV1::DuplicateSlot { item_index: 1 })
    ));
}

#[test]
fn applies_multiple_items_at_the_last_valid_project_revision() {
    let mut basis = basis();
    basis.revision = u64::MAX - 1;
    let requests = [request(3, 4, 5, "de"), request(7, 8, 9, "de")]
        .into_iter()
        .map(|json| {
            let mut request: Revision3VoiceTakeStageRequestV1 =
                serde_json::from_str(&json).unwrap();
            request.expected_revision = u64::MAX - 1;
            request.to_canonical_json().unwrap()
        })
        .collect::<Vec<_>>();
    let receipts = vec![
        imported(0x41, "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg"),
        imported(0x42, "VLK_574_VIPER_HELLO_01_01.ogg"),
    ];

    let Revision3VoiceTakeBatchEvaluationV1::Applied(outcome) =
        apply_revision3_voice_take_batch_transaction_v1(
            &head(),
            &basis.to_canonical_json().unwrap(),
            &requests,
            receipts,
        )
        .unwrap()
    else {
        panic!("expected the last representable batch revision to apply")
    };
    assert_eq!(outcome.project.revision, u64::MAX);
    assert_eq!(outcome.items.len(), 2);
}

#[test]
fn rejects_legal_item_count_when_project_work_product_is_too_large() {
    let mut basis = basis();
    let EntityPayload::LocalizationEntry(localization) =
        &mut basis.entities.get_mut(&id(2)).unwrap().payload
    else {
        unreachable!()
    };
    localization
        .texts
        .insert(locale("de"), "x".repeat(256 * 1024));
    let basis_json = basis.to_canonical_json().unwrap();
    let requests = vec![request(3, 4, 5, "de"); 256];
    let receipts = vec![imported(0x41, "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg"); 256];

    let evaluation =
        apply_revision3_voice_take_batch_transaction_v1(&head(), &basis_json, &requests, receipts)
            .unwrap();
    assert!(matches!(
        evaluation,
        Revision3VoiceTakeBatchEvaluationV1::Rejected(ref rejection)
            if matches!(
                rejection.conflict,
                Revision3VoiceTakeBatchConflictV1::ProjectWorkLimitExceeded {
                    project_bytes,
                    items: 256,
                    ..
                } if project_bytes == basis_json.len()
            )
    ));
}
