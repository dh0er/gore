use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::model_revision3::{
    DialogLine, Entity, EntityKind, EntityPayload, LocalizationEntry, OriginRef, SchemaRevisionV3,
    TypedRef, VoiceTakeStatus, VoiceTargetResolution,
};
use gore_authoring::{
    apply_revision3_dialog_voice_slot_creation_transaction_v1,
    apply_revision3_dialog_voice_slot_removal_transaction_v1,
    apply_revision3_voice_take_transaction_v1, AssetMeta, AssetRef, AssetStoreIndex, ContentSeal,
    EntityId, FormatV2, GameGenerationAnchor, ImportedOgg, LocaleCode, OggCodec, OggMetadata,
    ProjectId, ProjectMeta, ProjectRevision3, Revision3ContentReferenceRoleV1,
    Revision3DialogVoiceSlotCreationBuildStatusV1, Revision3DialogVoiceSlotCreationConflictV1,
    Revision3DialogVoiceSlotCreationErrorV1, Revision3DialogVoiceSlotCreationEvaluationV1,
    Revision3DialogVoiceSlotCreationOutcomeV1, Revision3DialogVoiceSlotCreationPublicationStatusV1,
    Revision3DialogVoiceSlotCreationRequestJsonErrorV1, Revision3DialogVoiceSlotCreationRequestV1,
    Revision3DialogVoiceSlotCreationRuntimeStatusV1,
    Revision3DialogVoiceSlotCreationTargetAuthorityV1, Revision3DialogVoiceSlotRemovalEvaluationV1,
    Revision3DialogVoiceSlotRemovalRequestV1, Revision3VoiceTakeStageEvaluationV1,
    Revision3VoiceTakeStageRequestV1, Sha256Digest, WorkingHead, WorkingStoreFormat,
    MAX_REVISION3_DIALOG_VOICE_SLOT_CREATION_REQUEST_JSON_BYTES_V1,
    REVISION3_VOICE_SLOT_GENERATOR_ID_V1, REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1,
};

fn project_id(tag: u8) -> ProjectId {
    ProjectId::from_bytes([tag; 16])
}

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

fn locale(value: &str) -> LocaleCode {
    value.parse().unwrap()
}

fn new_origin(value: &str) -> OriginRef {
    OriginRef::New {
        authored_runtime_id: value.to_owned(),
    }
}

fn basis() -> ProjectRevision3 {
    let project_id = project_id(0x10);
    let localization_id = id(2);
    let line_id = id(3);
    let unrelated_id = id(5);
    let de = locale("de");
    let en = locale("en");
    let mut assets = AssetStoreIndex::default();
    assets.assets.insert(
        digest(0x90),
        AssetMeta {
            byte_len: 17,
            media_type: "application/octet-stream".to_owned(),
        },
    );
    ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id,
        revision: 7,
        meta: ProjectMeta {
            name: "Dialog VoiceSlot creation".to_owned(),
            version: "1.0.0".to_owned(),
            author: "tests".to_owned(),
        },
        target: target(0x20),
        authoring_locales: BTreeSet::from([de.clone(), en.clone()]),
        entities: BTreeMap::from([
            (
                localization_id,
                Entity {
                    id: localization_id,
                    display_name: "Asghan line text".to_owned(),
                    origin: new_origin("GORE_ASGHAN_GREETING_TEXT"),
                    revision: 2,
                    payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                        loc_id: "GORE_ASGHAN_GREETING".to_owned(),
                        texts: BTreeMap::from([
                            (de, "Willkommen.".to_owned()),
                            (en, "Welcome.".to_owned()),
                        ]),
                    }),
                },
            ),
            (
                line_id,
                Entity {
                    id: line_id,
                    display_name: "Asghan greeting".to_owned(),
                    origin: new_origin("DIA_GORE_ASGHAN_GREETING"),
                    revision: 3,
                    payload: EntityPayload::DialogLine(DialogLine {
                        localization: TypedRef::new(
                            project_id,
                            localization_id,
                            EntityKind::LocalizationEntry,
                        ),
                        speaker_hint: Some("Asghan".to_owned()),
                        voice_slots: BTreeMap::new(),
                    }),
                },
            ),
            (
                unrelated_id,
                Entity {
                    id: unrelated_id,
                    display_name: "Unrelated text".to_owned(),
                    origin: new_origin("GORE_UNRELATED_TEXT"),
                    revision: 9,
                    payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                        loc_id: "GORE_UNRELATED".to_owned(),
                        texts: BTreeMap::from([(locale("de"), "Bleibt unveraendert.".to_owned())]),
                    }),
                },
            ),
        ]),
        asset_store: assets,
    }
}

fn request() -> Revision3DialogVoiceSlotCreationRequestV1 {
    Revision3DialogVoiceSlotCreationRequestV1 {
        expected_head: head(0x31),
        expected_project_id: project_id(0x10),
        expected_revision: 7,
        expected_target: target(0x20),
        line_id: id(3),
        expected_line_revision: 3,
        localization_id: id(2),
        expected_loc_id: "GORE_ASGHAN_GREETING".to_owned(),
        locale: locale("de"),
        slot_id: id(4),
    }
}

fn evaluate_with_head(
    project: &ProjectRevision3,
    request: &Revision3DialogVoiceSlotCreationRequestV1,
    exact_head: &WorkingHead,
) -> Result<Revision3DialogVoiceSlotCreationEvaluationV1, Revision3DialogVoiceSlotCreationErrorV1> {
    apply_revision3_dialog_voice_slot_creation_transaction_v1(
        exact_head,
        &project.to_canonical_json().unwrap(),
        &request.to_canonical_json().unwrap(),
    )
}

fn evaluate(
    project: &ProjectRevision3,
    request: &Revision3DialogVoiceSlotCreationRequestV1,
) -> Revision3DialogVoiceSlotCreationEvaluationV1 {
    evaluate_with_head(project, request, &head(0x31)).unwrap()
}

fn applied(
    evaluation: Revision3DialogVoiceSlotCreationEvaluationV1,
) -> Revision3DialogVoiceSlotCreationOutcomeV1 {
    match evaluation {
        Revision3DialogVoiceSlotCreationEvaluationV1::Applied(outcome) => *outcome,
        Revision3DialogVoiceSlotCreationEvaluationV1::Rejected(rejection) => {
            panic!("unexpected rejection: {}", rejection.conflict)
        }
    }
}

fn conflict(
    evaluation: Revision3DialogVoiceSlotCreationEvaluationV1,
) -> Revision3DialogVoiceSlotCreationConflictV1 {
    match evaluation {
        Revision3DialogVoiceSlotCreationEvaluationV1::Rejected(rejection) => rejection.conflict,
        Revision3DialogVoiceSlotCreationEvaluationV1::Applied(_) => panic!("unexpected candidate"),
    }
}

#[test]
fn exact_empty_slot_and_line_edge_are_created_atomically() {
    let project = basis();
    let request = request();
    let outcome = applied(evaluate(&project, &request));
    let repeated = applied(evaluate(&project, &request));

    assert_eq!(
        outcome.canonical_project_json,
        repeated.canonical_project_json
    );
    assert_eq!(outcome.project.revision, 8);
    assert_eq!(outcome.basis_head, head(0x31));
    assert_eq!(outcome.line_id, id(3));
    assert_eq!(outcome.line_revision, 4);
    assert_eq!(outcome.localization_id, id(2));
    assert_eq!(outcome.localization_revision, 2);
    assert_eq!(outcome.slot_id, id(4));
    assert_eq!(outcome.slot_revision, 0);
    assert_eq!(outcome.locale, locale("de"));
    assert_eq!(outcome.loc_id, "GORE_ASGHAN_GREETING");
    assert_eq!(
        outcome.build_status,
        Revision3DialogVoiceSlotCreationBuildStatusV1::Blocked
    );
    assert_eq!(
        outcome.runtime_status,
        Revision3DialogVoiceSlotCreationRuntimeStatusV1::RuntimeUnqualified
    );
    assert_eq!(
        outcome.target_authority,
        Revision3DialogVoiceSlotCreationTargetAuthorityV1::NotGranted
    );
    assert_eq!(
        outcome.publication_status,
        Revision3DialogVoiceSlotCreationPublicationStatusV1::NotSupported
    );

    assert_eq!(outcome.project.meta, project.meta);
    assert_eq!(outcome.project.target, project.target);
    assert_eq!(outcome.project.authoring_locales, project.authoring_locales);
    assert_eq!(outcome.project.asset_store, project.asset_store);
    assert_eq!(outcome.project.entities[&id(2)], project.entities[&id(2)]);
    assert_eq!(outcome.project.entities[&id(5)], project.entities[&id(5)]);
    let EntityPayload::DialogLine(line) = &outcome.project.entities[&id(3)].payload else {
        panic!("expected DialogLine")
    };
    assert_eq!(outcome.project.entities[&id(3)].revision, 4);
    assert_eq!(
        line.voice_slots[&locale("de")],
        TypedRef::new(project_id(0x10), id(4), EntityKind::VoiceSlot)
    );
    let slot_entity = &outcome.project.entities[&id(4)];
    assert_eq!(slot_entity.display_name, "Voice de");
    assert_eq!(slot_entity.revision, 0);
    assert!(matches!(
        &slot_entity.origin,
        OriginRef::Generated { generator_id, generator_version, owner }
            if generator_id == REVISION3_VOICE_SLOT_GENERATOR_ID_V1
                && *generator_version == REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1
                && owner == &TypedRef::new(project_id(0x10), id(3), EntityKind::DialogLine)
    ));
    let EntityPayload::VoiceSlot(slot) = &slot_entity.payload else {
        panic!("expected VoiceSlot")
    };
    assert_eq!(slot.locale, locale("de"));
    assert!(matches!(
        slot.target_resolution,
        VoiceTargetResolution::Unresolved
    ));
    assert!(slot.candidates.is_empty());
    assert!(slot.selected.is_none());
    assert_eq!(
        ProjectRevision3::from_json(&outcome.canonical_project_json).unwrap(),
        outcome.project
    );
}

#[test]
fn exact_basis_line_localization_and_locale_are_cas_bound() {
    let project = basis();
    assert_eq!(
        conflict(evaluate_with_head(&project, &request(), &head(0x32)).unwrap()),
        Revision3DialogVoiceSlotCreationConflictV1::CurrentHeadMismatch
    );

    let mut changed = request();
    changed.expected_project_id = project_id(0x11);
    assert!(matches!(
        conflict(evaluate(&project, &changed)),
        Revision3DialogVoiceSlotCreationConflictV1::ProjectIdentityMismatch { .. }
    ));
    changed = request();
    changed.expected_revision = 8;
    assert!(matches!(
        conflict(evaluate(&project, &changed)),
        Revision3DialogVoiceSlotCreationConflictV1::ProjectRevisionConflict { .. }
    ));
    changed = request();
    changed.expected_target = target(0x21);
    assert_eq!(
        conflict(evaluate(&project, &changed)),
        Revision3DialogVoiceSlotCreationConflictV1::ProjectTargetMismatch
    );
    changed = request();
    changed.expected_line_revision = 4;
    assert!(matches!(
        conflict(evaluate(&project, &changed)),
        Revision3DialogVoiceSlotCreationConflictV1::DialogLineRevisionConflict { .. }
    ));
    changed = request();
    changed.localization_id = id(5);
    assert!(matches!(
        conflict(evaluate(&project, &changed)),
        Revision3DialogVoiceSlotCreationConflictV1::InvalidLocalizationReference { .. }
    ));
    changed = request();
    changed.expected_loc_id = "GORE_OTHER".to_owned();
    assert!(matches!(
        conflict(evaluate(&project, &changed)),
        Revision3DialogVoiceSlotCreationConflictV1::LocalizationIdentityMismatch { .. }
    ));

    changed = request();
    changed.locale = locale("fr");
    assert!(matches!(
        conflict(evaluate(&project, &changed)),
        Revision3DialogVoiceSlotCreationConflictV1::VoiceSlotLocaleNotAuthorable { .. }
    ));
    let mut blank = project.clone();
    blank.authoring_locales.insert(locale("fr"));
    let EntityPayload::LocalizationEntry(localization) =
        &mut blank.entities.get_mut(&id(2)).unwrap().payload
    else {
        panic!("expected localization")
    };
    localization.texts.insert(locale("fr"), "  ".to_owned());
    assert!(matches!(
        conflict(evaluate(&blank, &changed)),
        Revision3DialogVoiceSlotCreationConflictV1::VoiceSlotLocaleHasNoText { .. }
    ));
}

#[test]
fn occupied_id_or_locale_and_preexisting_local_backlinks_fail_closed() {
    let project = basis();
    let created = applied(evaluate(&project, &request()));
    let mut second_request = request();
    second_request.expected_revision = 8;
    second_request.expected_line_revision = 4;
    second_request.slot_id = id(6);
    assert!(matches!(
        conflict(evaluate(&created.project, &second_request)),
        Revision3DialogVoiceSlotCreationConflictV1::VoiceSlotLocaleAlreadyLinked { .. }
    ));

    let mut collision = request();
    collision.slot_id = id(5);
    assert!(matches!(
        conflict(evaluate(&project, &collision)),
        Revision3DialogVoiceSlotCreationConflictV1::VoiceSlotIdCollision { .. }
    ));

    let mut local_backlink = project.clone();
    local_backlink.entities.get_mut(&id(5)).unwrap().origin = OriginRef::Generated {
        generator_id: "tests.reference".to_owned(),
        generator_version: 1,
        owner: TypedRef::new(project_id(0x10), id(4), EntityKind::VoiceSlot),
    };
    assert!(matches!(
        conflict(evaluate(&local_backlink, &request())),
        Revision3DialogVoiceSlotCreationConflictV1::InvalidLocalBacklink {
            source_entity,
            role: Revision3ContentReferenceRoleV1::OriginOwner,
            ..
        } if source_entity == id(5)
    ));

    local_backlink.entities.get_mut(&id(5)).unwrap().origin = OriginRef::Generated {
        generator_id: "tests.reference".to_owned(),
        generator_version: 1,
        owner: TypedRef::new(project_id(0x11), id(4), EntityKind::VoiceSlot),
    };
    assert!(matches!(
        evaluate(&local_backlink, &request()),
        Revision3DialogVoiceSlotCreationEvaluationV1::Applied(_)
    ));
}

#[test]
fn invalid_identity_and_revision_overflow_reject_without_candidate() {
    let project = basis();
    let mut invalid = request();
    invalid.slot_id = EntityId::from_bytes([0; 16]);
    assert_eq!(
        conflict(evaluate(&project, &invalid)),
        Revision3DialogVoiceSlotCreationConflictV1::InvalidEntityIdentity
    );
    invalid = request();
    invalid.slot_id = invalid.line_id;
    assert_eq!(
        conflict(evaluate(&project, &invalid)),
        Revision3DialogVoiceSlotCreationConflictV1::InvalidEntityIdentity
    );
    invalid = request();
    invalid.expected_loc_id = "not/a/portable/id".to_owned();
    assert_eq!(
        conflict(evaluate(&project, &invalid)),
        Revision3DialogVoiceSlotCreationConflictV1::InvalidExpectedLocId
    );

    let mut overflow = project.clone();
    overflow.revision = u64::MAX;
    invalid = request();
    invalid.expected_revision = u64::MAX;
    assert_eq!(
        conflict(evaluate(&overflow, &invalid)),
        Revision3DialogVoiceSlotCreationConflictV1::ProjectRevisionOverflow
    );
    overflow = project.clone();
    overflow.entities.get_mut(&id(3)).unwrap().revision = u64::MAX;
    invalid = request();
    invalid.expected_line_revision = u64::MAX;
    assert!(matches!(
        conflict(evaluate(&overflow, &invalid)),
        Revision3DialogVoiceSlotCreationConflictV1::DialogLineRevisionOverflow { .. }
    ));
}

#[test]
fn creation_composes_with_take_staging_and_exact_empty_slot_removal() {
    let created = applied(evaluate(&basis(), &request()));
    let voice_request = Revision3VoiceTakeStageRequestV1 {
        expected_head: head(0x32),
        expected_project_id: project_id(0x10),
        expected_revision: 8,
        expected_target: target(0x20),
        line_id: id(3),
        slot_id: id(4),
        take_id: id(6),
        locale: locale("de"),
        text: None,
        take_display_name: "Asghan DE Take 1".to_owned(),
        logical_name: "GORE_ASGHAN_GREETING.ogg".to_owned(),
        status: VoiceTakeStatus::Recorded,
        select_take: false,
    };
    let imported = ImportedOgg {
        asset: AssetRef {
            sha256: digest(0x41),
            byte_len: 8192,
            logical_name: voice_request.logical_name.clone(),
        },
        ogg: OggMetadata {
            codec: OggCodec::Vorbis,
            channels: 1,
            sample_rate: 48_000,
            pages: 3,
            logical_streams: 1,
        },
        deduplicated: false,
    };
    let voice = apply_revision3_voice_take_transaction_v1(
        &head(0x32),
        &created.canonical_project_json,
        &voice_request.to_canonical_json().unwrap(),
        imported,
    )
    .unwrap();
    let Revision3VoiceTakeStageEvaluationV1::Applied(voice) = voice else {
        panic!("expected Voice take to compose with created empty slot")
    };
    assert!(!voice.slot_created);
    assert_eq!(voice.project.entities[&id(3)].revision, 4);
    assert_eq!(voice.project.entities[&id(4)].revision, 1);

    let removal_request = Revision3DialogVoiceSlotRemovalRequestV1 {
        expected_head: head(0x33),
        expected_project_id: project_id(0x10),
        expected_revision: 8,
        expected_target: target(0x20),
        line_id: id(3),
        expected_line_revision: 4,
        localization_id: id(2),
        expected_loc_id: "GORE_ASGHAN_GREETING".to_owned(),
        locale: locale("de"),
        slot_id: id(4),
        expected_slot_revision: 0,
    };
    let removed = apply_revision3_dialog_voice_slot_removal_transaction_v1(
        &head(0x33),
        &created.canonical_project_json,
        &removal_request.to_canonical_json().unwrap(),
    )
    .unwrap();
    let Revision3DialogVoiceSlotRemovalEvaluationV1::Applied(removed) = removed else {
        panic!("expected empty created slot to compose with exact removal")
    };
    assert_eq!(removed.project.revision, 9);
    assert_eq!(removed.project.entities[&id(3)].revision, 5);
    assert!(!removed.project.entities.contains_key(&id(4)));
}

#[test]
fn request_json_is_exact_canonical_duplicate_free_and_bounded() {
    let request = request();
    let canonical = request.to_canonical_json().unwrap();
    assert_eq!(
        Revision3DialogVoiceSlotCreationRequestV1::from_json(&canonical).unwrap(),
        request
    );

    let duplicate = canonical.replacen(
        "\"expected_revision\":7",
        "\"expected_revision\":7,\"expected_revision\":7",
        1,
    );
    assert!(matches!(
        Revision3DialogVoiceSlotCreationRequestV1::from_json(&duplicate),
        Err(Revision3DialogVoiceSlotCreationRequestJsonErrorV1::InvalidJson(_))
    ));
    let padded = format!("{canonical} ");
    assert!(matches!(
        Revision3DialogVoiceSlotCreationRequestV1::from_json(&padded),
        Err(Revision3DialogVoiceSlotCreationRequestJsonErrorV1::NonCanonicalJson)
    ));
    let oversized = " ".repeat(MAX_REVISION3_DIALOG_VOICE_SLOT_CREATION_REQUEST_JSON_BYTES_V1 + 1);
    assert!(matches!(
        Revision3DialogVoiceSlotCreationRequestV1::from_json(&oversized),
        Err(Revision3DialogVoiceSlotCreationRequestJsonErrorV1::InputTooLarge { .. })
    ));
}
