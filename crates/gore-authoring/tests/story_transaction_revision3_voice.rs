use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use gore_authoring::model_revision3::{
    DialogLine, Entity, EntityKind, EntityPayload, LocalizationEntry, OriginRef, SchemaRevisionV3,
    TypedRef, VoiceSlot, VoiceTake, VoiceTakeStatus, VoiceTarget, VoiceTargetResolution,
};
use gore_authoring::{
    apply_revision3_voice_take_transaction_v1, preflight_revision3_voice_take_transaction_v1,
    AssetMeta, AssetRef, AssetStoreIndex, ContentSeal, EntityId, FormatV2, GameGenerationAnchor,
    ImportedOgg, LocaleCode, OggCodec, OggMetadata, ProjectId, ProjectMeta, ProjectRevision3,
    Revision3VoiceBuildStatusV1, Revision3VoicePublicationStatusV1, Revision3VoiceRuntimeStatusV1,
    Revision3VoiceTakePreflightEvaluationV1, Revision3VoiceTakeStageConflictV1,
    Revision3VoiceTakeStageEvaluationV1, Revision3VoiceTakeStageRequestJsonErrorV1,
    Revision3VoiceTakeStageRequestV1, Revision3VoiceTargetAuthorityV1, Sha256Digest, WorkingHead,
    WorkingProjectStore, WorkingStoreFormat, WorkingStoreLimits, MAX_REVISION3_ASSETS,
    MAX_REVISION3_REFERENCED_ASSET_BYTES, REVISION3_VOICE_SLOT_GENERATOR_ID_V1,
    REVISION3_VOICE_TAKE_IMPORTER_ID_V1,
};

fn project_id() -> ProjectId {
    ProjectId::from_bytes([0x10; 16])
}

fn id(tag: u8) -> EntityId {
    EntityId::from_bytes([tag; 16])
}

fn digest(tag: u8) -> Sha256Digest {
    Sha256Digest::from_bytes([tag; 32])
}

fn numbered_digest(value: usize) -> Sha256Digest {
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&(value as u64).to_le_bytes());
    bytes[31] = 0xa5;
    Sha256Digest::from_bytes(bytes)
}

fn seal(tag: u8, byte_len: u64) -> ContentSeal {
    ContentSeal {
        byte_len,
        sha256: digest(tag),
    }
}

fn target() -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: seal(0x21, 171_698_176),
    }
}

fn head(tag: u8) -> WorkingHead {
    WorkingHead {
        store_format: WorkingStoreFormat,
        snapshot: seal(tag, 4096),
    }
}

fn locale() -> LocaleCode {
    "de".parse().unwrap()
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

fn imported_origin(tag: u8) -> OriginRef {
    OriginRef::Imported {
        importer: "tests".to_owned(),
        source_seal: seal(tag, 10),
        external_identity: None,
    }
}

fn basis() -> ProjectRevision3 {
    let localization_id = id(2);
    let line_id = id(3);
    ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id: project_id(),
        revision: 7,
        meta: ProjectMeta {
            name: "Voice transaction".into(),
            version: "1.0.0".into(),
            author: "tests".into(),
        },
        target: target(),
        authoring_locales: BTreeSet::new(),
        entities: BTreeMap::from([
            (
                localization_id,
                Entity {
                    id: localization_id,
                    display_name: "Asghan line text".into(),
                    origin: imported_origin(2),
                    revision: 4,
                    payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                        loc_id: "GRD_263_ASGHAN_OPEN_INFO_06_02".into(),
                        texts: BTreeMap::new(),
                    }),
                },
            ),
            (
                line_id,
                Entity {
                    id: line_id,
                    display_name: "Asghan greeting".into(),
                    origin: imported_origin(3),
                    revision: 2,
                    payload: EntityPayload::DialogLine(DialogLine {
                        localization: TypedRef::new(
                            project_id(),
                            localization_id,
                            EntityKind::LocalizationEntry,
                        ),
                        speaker_hint: Some("Asghan".into()),
                        voice_slots: BTreeMap::new(),
                    }),
                },
            ),
        ]),
        asset_store: AssetStoreIndex::default(),
    }
}

fn basis_with_selected_take(status: VoiceTakeStatus) -> ProjectRevision3 {
    let mut project = basis();
    let selected_take_id = id(6);
    let selected_asset = digest(0x42);
    project.authoring_locales.insert(locale());
    project.asset_store.assets.insert(
        selected_asset,
        AssetMeta {
            byte_len: 4096,
            media_type: "audio/ogg".into(),
        },
    );
    project.entities.insert(
        selected_take_id,
        Entity {
            id: selected_take_id,
            display_name: "Existing selected take".into(),
            origin: imported_origin(6),
            revision: 1,
            payload: EntityPayload::VoiceTake(VoiceTake {
                locale: locale(),
                asset: AssetRef {
                    sha256: selected_asset,
                    byte_len: 4096,
                    logical_name: "old.ogg".into(),
                },
                ogg: gore_authoring::Revision2OggMetadata {
                    codec: gore_authoring::Revision2OggCodec::Vorbis,
                    channels: 1,
                    sample_rate: 48_000,
                    pages: 2,
                    logical_streams: 1,
                },
                status,
            }),
        },
    );
    let selected_ref = TypedRef::new(project_id(), selected_take_id, EntityKind::VoiceTake);
    project.entities.insert(
        id(4),
        Entity {
            id: id(4),
            display_name: "Voice de".into(),
            origin: imported_origin(4),
            revision: 9,
            payload: EntityPayload::VoiceSlot(VoiceSlot {
                locale: locale(),
                target_resolution: VoiceTargetResolution::Unresolved,
                candidates: vec![selected_ref.clone()],
                selected: Some(selected_ref),
            }),
        },
    );
    let EntityPayload::DialogLine(line) = &mut project.entities.get_mut(&id(3)).unwrap().payload
    else {
        unreachable!()
    };
    line.voice_slots.insert(
        locale(),
        TypedRef::new(project_id(), id(4), EntityKind::VoiceSlot),
    );
    project
}

fn request(status: VoiceTakeStatus, select_take: bool) -> Revision3VoiceTakeStageRequestV1 {
    Revision3VoiceTakeStageRequestV1 {
        expected_head: head(0x31),
        expected_project_id: project_id(),
        expected_revision: 7,
        expected_target: target(),
        line_id: id(3),
        slot_id: id(4),
        take_id: id(5),
        locale: locale(),
        text: None,
        take_display_name: "Asghan DE Take 1".into(),
        logical_name: "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg".into(),
        status,
        select_take,
    }
}

fn apply(
    project: &ProjectRevision3,
    request: &Revision3VoiceTakeStageRequestV1,
    imported: ImportedOgg,
) -> Revision3VoiceTakeStageEvaluationV1 {
    apply_revision3_voice_take_transaction_v1(
        &head(0x31),
        &project.to_canonical_json().unwrap(),
        &request.to_canonical_json().unwrap(),
        imported,
    )
    .unwrap()
}

#[test]
fn creates_unresolved_slot_and_recorded_take_without_changing_localization() {
    let project = basis();
    let request = request(VoiceTakeStatus::Recorded, false);
    let imported = imported(0x41, &request.logical_name);
    let Revision3VoiceTakeStageEvaluationV1::Applied(outcome) =
        apply(&project, &request, imported.clone())
    else {
        panic!("expected applied Voice transaction")
    };

    assert_eq!(outcome.project.revision, 8);
    assert_eq!(outcome.line_id, id(3));
    assert_eq!(outcome.localization_id, id(2));
    assert_eq!(outcome.slot_id, id(4));
    assert_eq!(outcome.take_id, id(5));
    assert_eq!(outcome.locale, locale());
    assert_eq!(outcome.status, VoiceTakeStatus::Recorded);
    assert!(outcome.slot_created);
    assert!(!outcome.selected);
    assert_eq!(outcome.imported_ogg, imported);
    assert_eq!(outcome.build_status, Revision3VoiceBuildStatusV1::Blocked);
    assert_eq!(
        outcome.runtime_status,
        Revision3VoiceRuntimeStatusV1::RuntimeUnqualified
    );
    assert_eq!(
        outcome.target_authority,
        Revision3VoiceTargetAuthorityV1::NotGranted
    );
    assert_eq!(
        outcome.publication_status,
        Revision3VoicePublicationStatusV1::NotSupported
    );

    assert!(outcome.project.authoring_locales.contains(&locale()));
    let EntityPayload::LocalizationEntry(localization) = &outcome.project.entities[&id(2)].payload
    else {
        panic!("expected LocalizationEntry")
    };
    assert!(localization.texts.is_empty());
    assert_eq!(outcome.project.entities[&id(2)].revision, 4);

    let EntityPayload::DialogLine(line) = &outcome.project.entities[&id(3)].payload else {
        panic!("expected DialogLine")
    };
    assert_eq!(line.voice_slots[&locale()].id, id(4));
    assert_eq!(outcome.project.entities[&id(3)].revision, 3);

    let slot_entity = &outcome.project.entities[&id(4)];
    let EntityPayload::VoiceSlot(slot) = &slot_entity.payload else {
        panic!("expected VoiceSlot")
    };
    assert!(matches!(
        slot.target_resolution,
        VoiceTargetResolution::Unresolved
    ));
    assert_eq!(
        slot.candidates,
        vec![TypedRef::new(project_id(), id(5), EntityKind::VoiceTake)]
    );
    assert_eq!(slot.selected, None);
    assert!(matches!(
        &slot_entity.origin,
        OriginRef::Generated { generator_id, owner, .. }
            if generator_id == REVISION3_VOICE_SLOT_GENERATOR_ID_V1
                && owner.id == id(3)
                && owner.expected_kind == EntityKind::DialogLine
    ));

    let take_entity = &outcome.project.entities[&id(5)];
    let EntityPayload::VoiceTake(take) = &take_entity.payload else {
        panic!("expected VoiceTake")
    };
    assert_eq!(take.locale, locale());
    assert_eq!(take.asset.sha256, digest(0x41));
    assert_eq!(take.status, VoiceTakeStatus::Recorded);
    assert!(matches!(
        &take_entity.origin,
        OriginRef::Imported { importer, source_seal, external_identity }
            if importer == REVISION3_VOICE_TAKE_IMPORTER_ID_V1
                && source_seal.sha256 == digest(0x41)
                && external_identity.is_none()
    ));
    assert_eq!(
        outcome.project.asset_store.assets[&digest(0x41)],
        AssetMeta {
            byte_len: 8192,
            media_type: "audio/ogg".into(),
        }
    );
    assert_eq!(
        ProjectRevision3::from_json(&outcome.canonical_project_json).unwrap(),
        outcome.project
    );
}

#[test]
fn explicit_localization_edit_changes_text_and_bumps_only_once() {
    let project = basis();
    let mut request = request(VoiceTakeStatus::Recorded, false);
    request.text = Some("Du willst in die Mine?".into());
    let Revision3VoiceTakeStageEvaluationV1::Applied(outcome) =
        apply(&project, &request, imported(0x44, &request.logical_name))
    else {
        panic!("expected applied Voice transaction")
    };
    let EntityPayload::LocalizationEntry(localization) = &outcome.project.entities[&id(2)].payload
    else {
        panic!("expected LocalizationEntry")
    };
    assert_eq!(
        localization.texts.get(&locale()).map(String::as_str),
        Some("Du willst in die Mine?")
    );
    assert_eq!(outcome.project.entities[&id(2)].revision, 5);
}

#[test]
fn pure_preflight_rejects_semantic_drift_without_an_ogg_receipt() {
    let project = basis();
    let ready = request(VoiceTakeStatus::Recorded, false);
    assert!(matches!(
        preflight_revision3_voice_take_transaction_v1(
            &head(0x31),
            &project.to_canonical_json().unwrap(),
            &ready.to_canonical_json().unwrap(),
        )
        .unwrap(),
        Revision3VoiceTakePreflightEvaluationV1::Ready
    ));

    let invalid = request(VoiceTakeStatus::Reviewed, true);
    assert!(matches!(
        preflight_revision3_voice_take_transaction_v1(
            &head(0x31),
            &project.to_canonical_json().unwrap(),
            &invalid.to_canonical_json().unwrap(),
        )
        .unwrap(),
        Revision3VoiceTakePreflightEvaluationV1::Rejected(rejection)
            if rejection.conflict == Revision3VoiceTakeStageConflictV1::UnapprovedTakeSelection
    ));
}

#[test]
fn non_voice_dialog_loc_id_stays_valid_but_cannot_gain_an_unsafe_voice_slot() {
    let mut project = basis();
    let EntityPayload::LocalizationEntry(localization) =
        &mut project.entities.get_mut(&id(2)).unwrap().payload
    else {
        unreachable!()
    };
    localization.loc_id = "CON".to_owned();

    // General dialog/localization authoring is not narrowed merely because this LocID cannot be
    // represented as one portable `<LocID>.ogg` archive basename.
    project.validate_closed_model().unwrap();

    let request = request(VoiceTakeStatus::Recorded, false);
    assert!(matches!(
        preflight_revision3_voice_take_transaction_v1(
            &head(0x31),
            &project.to_canonical_json().unwrap(),
            &request.to_canonical_json().unwrap(),
        )
        .unwrap(),
        Revision3VoiceTakePreflightEvaluationV1::Rejected(rejection)
            if rejection.conflict
                == Revision3VoiceTakeStageConflictV1::InvalidLocalizationReference {
                    line: id(3)
                }
    ));
}

#[test]
fn appends_and_selects_only_an_approved_take_on_the_exact_existing_slot() {
    let mut project = basis();
    let old_take_id = id(6);
    let old_asset = digest(0x42);
    project.authoring_locales.insert(locale());
    project.asset_store.assets.insert(
        old_asset,
        AssetMeta {
            byte_len: 4096,
            media_type: "audio/ogg".into(),
        },
    );
    project.entities.insert(
        old_take_id,
        Entity {
            id: old_take_id,
            display_name: "Approved take".into(),
            origin: imported_origin(6),
            revision: 1,
            payload: EntityPayload::VoiceTake(VoiceTake {
                locale: locale(),
                asset: AssetRef {
                    sha256: old_asset,
                    byte_len: 4096,
                    logical_name: "old.ogg".into(),
                },
                ogg: gore_authoring::Revision2OggMetadata {
                    codec: gore_authoring::Revision2OggCodec::Vorbis,
                    channels: 1,
                    sample_rate: 48_000,
                    pages: 2,
                    logical_streams: 1,
                },
                status: VoiceTakeStatus::Approved,
            }),
        },
    );
    let old_ref = TypedRef::new(project_id(), old_take_id, EntityKind::VoiceTake);
    project.entities.insert(
        id(4),
        Entity {
            id: id(4),
            display_name: "Voice de".into(),
            origin: imported_origin(4),
            revision: 9,
            payload: EntityPayload::VoiceSlot(VoiceSlot {
                locale: locale(),
                target_resolution: VoiceTargetResolution::Unresolved,
                candidates: vec![old_ref.clone()],
                selected: Some(old_ref),
            }),
        },
    );
    let EntityPayload::DialogLine(line) = &mut project.entities.get_mut(&id(3)).unwrap().payload
    else {
        unreachable!()
    };
    line.voice_slots.insert(
        locale(),
        TypedRef::new(project_id(), id(4), EntityKind::VoiceSlot),
    );
    let EntityPayload::LocalizationEntry(localization) =
        &mut project.entities.get_mut(&id(2)).unwrap().payload
    else {
        unreachable!()
    };
    localization
        .texts
        .insert(locale(), "Du willst in die Mine?".into());

    let request = request(VoiceTakeStatus::Approved, true);
    let Revision3VoiceTakeStageEvaluationV1::Applied(outcome) =
        apply(&project, &request, imported(0x43, &request.logical_name))
    else {
        panic!("expected applied Voice transaction")
    };
    assert!(!outcome.slot_created);
    assert!(outcome.selected);
    let EntityPayload::VoiceSlot(slot) = &outcome.project.entities[&id(4)].payload else {
        panic!("expected VoiceSlot")
    };
    assert_eq!(slot.candidates.len(), 2);
    assert_eq!(slot.selected.as_ref().map(|value| value.id), Some(id(5)));
    assert_eq!(outcome.project.entities[&id(4)].revision, 10);
    assert_eq!(outcome.project.entities[&id(3)].revision, 2);
    assert_eq!(outcome.project.entities[&id(2)].revision, 4);
}

#[test]
fn appends_to_existing_slot_with_selected_reviewed_take_without_selecting_new_take() {
    let project = basis_with_selected_take(VoiceTakeStatus::Reviewed);
    project.validate_closed_model().unwrap();

    let request = request(VoiceTakeStatus::Recorded, false);
    let Revision3VoiceTakeStageEvaluationV1::Applied(outcome) =
        apply(&project, &request, imported(0x43, &request.logical_name))
    else {
        panic!("selected non-approved history must not prevent adding another unselected take")
    };
    assert!(!outcome.slot_created);
    assert!(!outcome.selected);
    let EntityPayload::VoiceSlot(slot) = &outcome.project.entities[&id(4)].payload else {
        panic!("expected VoiceSlot")
    };
    assert_eq!(slot.candidates.len(), 2);
    assert_eq!(slot.selected.as_ref().map(|value| value.id), Some(id(6)));
    outcome.project.validate_closed_model().unwrap();
}

#[test]
fn strict_request_and_semantic_boundary_reject_drift_and_authority_escalation() {
    let canonical = request(VoiceTakeStatus::Recorded, false)
        .to_canonical_json()
        .unwrap();
    assert!(matches!(
        Revision3VoiceTakeStageRequestV1::from_json(&format!(" {canonical}")),
        Err(Revision3VoiceTakeStageRequestJsonErrorV1::NonCanonicalJson)
    ));

    let project = basis();
    let mut invalid_text = request(VoiceTakeStatus::Recorded, false);
    invalid_text.text = Some("  ".into());
    assert!(matches!(
        apply(
            &project,
            &invalid_text,
            imported(0x50, &invalid_text.logical_name)
        ),
        Revision3VoiceTakeStageEvaluationV1::Rejected(rejection)
            if rejection.conflict == Revision3VoiceTakeStageConflictV1::InvalidLocalizedText
    ));

    for logical_name in [
        "../x.ogg",
        r"dir\x.ogg",
        "C:x.ogg",
        "CON.ogg",
        "Lpt1.OGG",
        " x.ogg",
        "x.ogg ",
        ".ogg",
        "x?.ogg",
    ] {
        let mut invalid_name = request(VoiceTakeStatus::Recorded, false);
        invalid_name.logical_name = logical_name.into();
        assert!(matches!(
            apply(
                &project,
                &invalid_name,
                imported(0x50, &invalid_name.logical_name)
            ),
            Revision3VoiceTakeStageEvaluationV1::Rejected(rejection)
                if rejection.conflict == Revision3VoiceTakeStageConflictV1::InvalidLogicalName
        ));
    }

    let mut valid_leaf = request(VoiceTakeStatus::Recorded, false);
    valid_leaf.logical_name = "Asghan Take 01.OGG".into();
    assert!(matches!(
        apply(
            &project,
            &valid_leaf,
            imported(0x50, &valid_leaf.logical_name)
        ),
        Revision3VoiceTakeStageEvaluationV1::Applied(_)
    ));

    let mut wrong_head = request(VoiceTakeStatus::Recorded, false);
    wrong_head.expected_head = head(0x99);
    assert!(matches!(
        apply(
            &project,
            &wrong_head,
            imported(0x51, &wrong_head.logical_name)
        ),
        Revision3VoiceTakeStageEvaluationV1::Rejected(rejection)
            if rejection.conflict == Revision3VoiceTakeStageConflictV1::CurrentHeadMismatch
    ));

    let unapproved = request(VoiceTakeStatus::Reviewed, true);
    assert!(matches!(
        apply(
            &project,
            &unapproved,
            imported(0x52, &unapproved.logical_name)
        ),
        Revision3VoiceTakeStageEvaluationV1::Rejected(rejection)
            if rejection.conflict == Revision3VoiceTakeStageConflictV1::UnapprovedTakeSelection
    ));

    let request = request(VoiceTakeStatus::Recorded, false);
    assert!(matches!(
        apply(&project, &request, imported(0x53, "different.ogg")),
        Revision3VoiceTakeStageEvaluationV1::Rejected(rejection)
            if rejection.conflict == Revision3VoiceTakeStageConflictV1::InvalidImportedOgg
    ));
}

#[test]
fn adding_future_takes_preserves_valid_resolved_and_ambiguous_targets() {
    let first = VoiceTarget {
        archive: "german.zip".into(),
        member: "NPC/Asghan/line.ogg".into(),
        operation: gore_authoring::Revision2VoiceOperation::Replace,
        archive_seal: gore_authoring::ArchiveSeal {
            byte_len: 100,
            sha256: digest(0x61),
        },
        member_proof: gore_authoring::Revision2VoiceMemberProof::Present {
            uncompressed_size: 10,
            crc32: 123,
        },
    };
    let mut second = first.clone();
    second.archive = "german_new.zip".into();
    second.archive_seal.sha256 = digest(0x62);

    for resolution in [
        VoiceTargetResolution::Resolved {
            target: first.clone(),
        },
        VoiceTargetResolution::Ambiguous {
            candidates: vec![first.clone(), second.clone()],
        },
    ] {
        let mut project = basis();
        let locale = locale();
        project.authoring_locales.insert(locale.clone());
        project.entities.insert(
            id(4),
            Entity {
                id: id(4),
                display_name: "Targeted voice".into(),
                origin: imported_origin(4),
                revision: 0,
                payload: EntityPayload::VoiceSlot(VoiceSlot {
                    locale: locale.clone(),
                    target_resolution: resolution.clone(),
                    candidates: Vec::new(),
                    selected: None,
                }),
            },
        );
        let EntityPayload::DialogLine(line) =
            &mut project.entities.get_mut(&id(3)).unwrap().payload
        else {
            unreachable!()
        };
        line.voice_slots.insert(
            locale,
            TypedRef::new(project_id(), id(4), EntityKind::VoiceSlot),
        );
        let request = request(VoiceTakeStatus::Recorded, false);
        let Revision3VoiceTakeStageEvaluationV1::Applied(outcome) =
            apply(&project, &request, imported(0x63, &request.logical_name))
        else {
            panic!("expected target-preserving VoiceTake transaction")
        };
        let EntityPayload::VoiceSlot(slot) = &outcome.project.entities[&id(4)].payload else {
            panic!("expected VoiceSlot")
        };
        assert_eq!(slot.target_resolution, resolution);
        assert_eq!(slot.candidates.len(), 1);
    }
}

#[test]
fn asset_count_and_aggregate_capacity_reject_during_pure_preview_evaluation() {
    let root = std::env::temp_dir().join(format!(
        "gore-authoring-voice-capacity-preview-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let source = root.join("take.ogg");
    fs::write(
        &source,
        include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
    )
    .unwrap();
    let store = WorkingProjectStore::at(&root, WorkingStoreLimits::default()).unwrap();
    let request = request(VoiceTakeStatus::Recorded, false);
    let prepared = store
        .prepare_ogg_import_classified(&source, request.logical_name.clone())
        .unwrap();
    let imported = prepared.preview();

    let mut count_exhausted = basis();
    for index in 0..MAX_REVISION3_ASSETS {
        count_exhausted.asset_store.assets.insert(
            numbered_digest(index),
            AssetMeta {
                byte_len: 1,
                media_type: "application/octet-stream".to_owned(),
            },
        );
    }
    assert!(matches!(
        apply(&count_exhausted, &request, imported.clone()),
        Revision3VoiceTakeStageEvaluationV1::Rejected(rejection)
            if rejection.conflict == Revision3VoiceTakeStageConflictV1::AssetCapacityExceeded
    ));

    let mut bytes_exhausted = basis();
    bytes_exhausted.asset_store.assets.insert(
        digest(0x72),
        AssetMeta {
            byte_len: MAX_REVISION3_REFERENCED_ASSET_BYTES - imported.asset.byte_len + 1,
            media_type: "application/octet-stream".to_owned(),
        },
    );
    assert!(matches!(
        apply(&bytes_exhausted, &request, imported),
        Revision3VoiceTakeStageEvaluationV1::Rejected(rejection)
            if rejection.conflict == Revision3VoiceTakeStageConflictV1::AssetCapacityExceeded
    ));
    assert!(!root.join("assets").exists());
    assert!(!root.join(".gore").join("staging").exists());
    fs::remove_dir_all(root).unwrap();
}
