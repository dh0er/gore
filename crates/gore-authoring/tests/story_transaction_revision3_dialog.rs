use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::model_revision3::{
    DialogLine, Entity, EntityKind, EntityPayload, LocalizationEntry, OriginRef, SchemaRevisionV3,
    TypedRef, VoiceTakeStatus, VoiceTargetResolution,
};
use gore_authoring::{
    apply_revision3_dialog_line_insert_transaction_v1, apply_revision3_voice_take_transaction_v1,
    AssetMeta, AssetRef, AssetStoreIndex, ContentSeal, EntityId, FormatV2, GameGenerationAnchor,
    ImportedOgg, LocaleCode, OggCodec, OggMetadata, ProjectId, ProjectMeta, ProjectRevision3,
    Revision3DialogBuildStatusV1, Revision3DialogEmptyVoiceSlotIntentV1,
    Revision3DialogEntityRoleV1, Revision3DialogLineInsertConflictV1,
    Revision3DialogLineInsertEvaluationV1, Revision3DialogLineInsertOutcomeV1,
    Revision3DialogLineInsertRequestJsonErrorV1, Revision3DialogLineInsertRequestV1,
    Revision3DialogLocalizationActionV1, Revision3DialogLocalizationIntentV1,
    Revision3DialogPublicationStatusV1, Revision3DialogRuntimeStatusV1,
    Revision3DialogTopicAuthorityV1, Revision3VoiceTakeStageEvaluationV1,
    Revision3VoiceTakeStageRequestV1, Sha256Digest, WorkingHead, WorkingStoreFormat,
    MAX_PROJECT_JSON_BYTES, MAX_REVISION3_DIALOG_LINE_REQUEST_JSON_BYTES_V1,
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

fn basis() -> ProjectRevision3 {
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
        project_id: project_id(0x10),
        revision: 7,
        meta: ProjectMeta {
            name: "Dialog transaction".to_owned(),
            version: "1.0.0".to_owned(),
            author: "tests".to_owned(),
        },
        target: target(0x20),
        authoring_locales: BTreeSet::new(),
        entities: BTreeMap::new(),
        asset_store: assets,
    }
}

fn create_request(with_slot: bool) -> Revision3DialogLineInsertRequestV1 {
    Revision3DialogLineInsertRequestV1 {
        expected_head: head(0x30),
        expected_project_id: project_id(0x10),
        expected_revision: 7,
        expected_target: target(0x20),
        line_id: id(1),
        line_display_name: "Welcome to the Old Camp".to_owned(),
        line_authored_identity: "DIA_GORE_ASGHAN_WELCOME".to_owned(),
        speaker_hint: Some("Asghan".to_owned()),
        localization: Revision3DialogLocalizationIntentV1::Create {
            localization_id: id(2),
            display_name: "Asghan welcome text".to_owned(),
            loc_id: "GORE_ASGHAN_WELCOME".to_owned(),
            texts: BTreeMap::from([
                (locale("de"), "Willkommen im Alten Lager.".to_owned()),
                (locale("en"), "Welcome to the Old Camp.".to_owned()),
            ]),
        },
        voice_slot: with_slot.then(|| Revision3DialogEmptyVoiceSlotIntentV1 {
            slot_id: id(3),
            locale: locale("de"),
            display_name: "Asghan welcome voice (German)".to_owned(),
        }),
    }
}

fn evaluate(
    project: &ProjectRevision3,
    request: &Revision3DialogLineInsertRequestV1,
) -> Revision3DialogLineInsertEvaluationV1 {
    apply_revision3_dialog_line_insert_transaction_v1(
        &head(0x30),
        &project.to_canonical_json().unwrap(),
        &request.to_canonical_json().unwrap(),
    )
    .unwrap()
}

fn applied(value: Revision3DialogLineInsertEvaluationV1) -> Revision3DialogLineInsertOutcomeV1 {
    match value {
        Revision3DialogLineInsertEvaluationV1::Applied(outcome) => *outcome,
        Revision3DialogLineInsertEvaluationV1::Rejected(rejection) => {
            panic!("unexpected rejection: {}", rejection.conflict)
        }
    }
}

fn rejected(value: Revision3DialogLineInsertEvaluationV1) -> Revision3DialogLineInsertConflictV1 {
    match value {
        Revision3DialogLineInsertEvaluationV1::Rejected(rejection) => rejection.conflict,
        Revision3DialogLineInsertEvaluationV1::Applied(_) => panic!("unexpected candidate"),
    }
}

#[test]
fn create_pair_with_empty_slot_is_deterministic_reopened_and_explicitly_unqualified() {
    let project = basis();
    let request = create_request(true);
    let first = applied(evaluate(&project, &request));
    let second = applied(evaluate(&project, &request));

    assert_eq!(first, second);
    assert_eq!(first.basis_head, head(0x30));
    assert_eq!(first.project.revision, 8);
    assert_eq!(first.project.entities.len(), 3);
    assert_eq!(first.localization_id, id(2));
    assert_eq!(first.voice_slot_id, Some(id(3)));
    assert_eq!(
        first.localization_action,
        Revision3DialogLocalizationActionV1::Created
    );
    assert_eq!(first.build_status, Revision3DialogBuildStatusV1::Blocked);
    assert_eq!(
        first.runtime_status,
        Revision3DialogRuntimeStatusV1::RuntimeUnqualified
    );
    assert_eq!(
        first.topic_authority,
        Revision3DialogTopicAuthorityV1::NotGranted
    );
    assert_eq!(
        first.publication_status,
        Revision3DialogPublicationStatusV1::NotSupported
    );
    assert_eq!(first.project.asset_store, project.asset_store);
    assert_eq!(first.project.meta, project.meta);
    assert_eq!(first.project.target, project.target);
    assert_eq!(
        ProjectRevision3::from_json(&first.canonical_project_json).unwrap(),
        first.project
    );

    let localization_entity = &first.project.entities[&id(2)];
    assert_eq!(localization_entity.revision, 0);
    assert!(matches!(
        &localization_entity.origin,
        OriginRef::New { authored_runtime_id } if authored_runtime_id == "GORE_ASGHAN_WELCOME"
    ));
    let EntityPayload::LocalizationEntry(localization) = &localization_entity.payload else {
        panic!("expected localization")
    };
    assert_eq!(
        localization.texts[&locale("de")],
        "Willkommen im Alten Lager."
    );

    let line_entity = &first.project.entities[&id(1)];
    assert_eq!(line_entity.revision, 0);
    assert!(matches!(
        &line_entity.origin,
        OriginRef::New { authored_runtime_id } if authored_runtime_id == "DIA_GORE_ASGHAN_WELCOME"
    ));
    let EntityPayload::DialogLine(line) = &line_entity.payload else {
        panic!("expected dialog line")
    };
    assert_eq!(line.localization.project_id, project.project_id);
    assert_eq!(line.localization.id, id(2));
    assert_eq!(
        line.localization.expected_kind,
        EntityKind::LocalizationEntry
    );
    assert_eq!(line.speaker_hint.as_deref(), Some("Asghan"));
    assert_eq!(line.voice_slots[&locale("de")].id, id(3));

    let slot_entity = &first.project.entities[&id(3)];
    assert_eq!(slot_entity.revision, 0);
    assert!(matches!(
        &slot_entity.origin,
        OriginRef::Generated {
            generator_id,
            generator_version,
            owner,
        } if generator_id == REVISION3_VOICE_SLOT_GENERATOR_ID_V1
            && *generator_version == REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1
            && owner == &TypedRef::new(project.project_id, id(1), EntityKind::DialogLine)
    ));
    let EntityPayload::VoiceSlot(slot) = &slot_entity.payload else {
        panic!("expected voice slot")
    };
    assert_eq!(slot.locale, locale("de"));
    assert_eq!(slot.target_resolution, VoiceTargetResolution::Unresolved);
    assert!(slot.candidates.is_empty());
    assert!(slot.selected.is_none());
    assert_eq!(
        first.project.authoring_locales,
        BTreeSet::from([locale("de"), locale("en")])
    );
}

#[test]
fn absent_speaker_and_slot_remain_absent_but_pair_is_voice_authorable() {
    let mut request = create_request(false);
    request.speaker_hint = None;
    let canonical_request = request.to_canonical_json().unwrap();
    assert!(!canonical_request.contains("speaker_hint"));
    assert!(!canonical_request.contains("voice_slot"));

    let outcome = applied(evaluate(&basis(), &request));
    assert_eq!(outcome.voice_slot_id, None);
    let EntityPayload::DialogLine(line) = &outcome.project.entities[&id(1)].payload else {
        panic!("expected line")
    };
    assert_eq!(line.speaker_hint, None);
    assert!(line.voice_slots.is_empty());
    assert!(outcome.project.validate_closed_model().is_ok());
}

#[test]
fn exact_existing_managed_localization_is_reused_byte_for_byte() {
    let mut project = basis();
    project.authoring_locales.insert(locale("de"));
    project.entities.insert(
        id(2),
        Entity {
            id: id(2),
            display_name: "Existing shared greeting".to_owned(),
            origin: OriginRef::Imported {
                importer: "tests".to_owned(),
                source_seal: seal(0x44, 77),
                external_identity: Some("legacy:greeting".to_owned()),
            },
            revision: 4,
            payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                loc_id: "GORE_SHARED_GREETING".to_owned(),
                texts: BTreeMap::from([(locale("de"), "Hallo.".to_owned())]),
            }),
        },
    );
    let before = project.entities[&id(2)].clone();
    let request = Revision3DialogLineInsertRequestV1 {
        expected_head: head(0x30),
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        expected_target: project.target.clone(),
        line_id: id(1),
        line_display_name: "Shared greeting line".to_owned(),
        line_authored_identity: "DIA_GORE_SHARED_GREETING".to_owned(),
        speaker_hint: None,
        localization: Revision3DialogLocalizationIntentV1::ReuseExact {
            localization_id: id(2),
            expected_localization_revision: 4,
            expected_loc_id: "GORE_SHARED_GREETING".to_owned(),
        },
        voice_slot: None,
    };

    let outcome = applied(evaluate(&project, &request));
    assert_eq!(
        outcome.localization_action,
        Revision3DialogLocalizationActionV1::ReusedExact
    );
    assert_eq!(outcome.project.entities.len(), project.entities.len() + 1);
    assert_eq!(outcome.project.entities[&id(2)], before);
    assert_eq!(
        serde_json::to_vec(&outcome.project.entities[&id(2)]).unwrap(),
        serde_json::to_vec(&project.entities[&id(2)]).unwrap()
    );
}

#[test]
fn empty_slot_composes_with_existing_voice_take_transaction() {
    let dialog = applied(evaluate(&basis(), &create_request(true)));
    let voice_head = head(0x31);
    let voice_request = Revision3VoiceTakeStageRequestV1 {
        expected_head: voice_head.clone(),
        expected_project_id: dialog.project.project_id,
        expected_revision: dialog.project.revision,
        expected_target: dialog.project.target.clone(),
        line_id: id(1),
        slot_id: id(3),
        take_id: id(4),
        locale: locale("de"),
        text: None,
        take_display_name: "Asghan take 1".to_owned(),
        logical_name: "GORE_ASGHAN_WELCOME.ogg".to_owned(),
        status: VoiceTakeStatus::Approved,
        select_take: true,
    };
    let imported = ImportedOgg {
        asset: AssetRef {
            sha256: digest(0x55),
            byte_len: 8192,
            logical_name: "GORE_ASGHAN_WELCOME.ogg".to_owned(),
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
        &voice_head,
        &dialog.canonical_project_json,
        &voice_request.to_canonical_json().unwrap(),
        imported,
    )
    .unwrap();
    let Revision3VoiceTakeStageEvaluationV1::Applied(voice) = voice else {
        panic!("created pair did not accept a Voice take")
    };
    assert!(!voice.slot_created);
    assert!(voice.selected);
    assert_eq!(voice.localization_id, id(2));
    let EntityPayload::VoiceSlot(slot) = &voice.project.entities[&id(3)].payload else {
        panic!("expected slot")
    };
    assert_eq!(slot.candidates.len(), 1);
    assert_eq!(slot.selected.as_ref().unwrap().id, id(4));
}

#[test]
fn request_wire_is_bounded_canonical_duplicate_safe_and_preserves_optional_shape() {
    let request = create_request(true);
    let canonical = request.to_canonical_json().unwrap();
    assert_eq!(
        Revision3DialogLineInsertRequestV1::from_json(&canonical).unwrap(),
        request
    );
    assert!(matches!(
        Revision3DialogLineInsertRequestV1::from_json(&(canonical.clone() + "\n")),
        Err(Revision3DialogLineInsertRequestJsonErrorV1::NonCanonicalJson)
    ));
    let duplicate = canonical.replacen(
        "\"expected_revision\":7",
        "\"expected_revision\":7,\"expected_revision\":7",
        1,
    );
    assert!(matches!(
        Revision3DialogLineInsertRequestV1::from_json(&duplicate),
        Err(Revision3DialogLineInsertRequestJsonErrorV1::InvalidJson(_))
    ));
    let oversized = "x".repeat(MAX_REVISION3_DIALOG_LINE_REQUEST_JSON_BYTES_V1 + 1);
    assert!(matches!(
        Revision3DialogLineInsertRequestV1::from_json(&oversized),
        Err(Revision3DialogLineInsertRequestJsonErrorV1::InputTooLarge { .. })
    ));
}

#[test]
fn exact_head_project_revision_target_and_overflow_conflicts_return_no_candidate() {
    let project = basis();
    let request = create_request(false);

    let mut changed = request.clone();
    changed.expected_head = head(0x99);
    assert_eq!(
        rejected(evaluate(&project, &changed)),
        Revision3DialogLineInsertConflictV1::CurrentHeadMismatch
    );

    changed = request.clone();
    changed.expected_project_id = project_id(0x99);
    assert!(matches!(
        rejected(evaluate(&project, &changed)),
        Revision3DialogLineInsertConflictV1::ProjectIdentityMismatch { .. }
    ));

    changed = request.clone();
    changed.expected_revision += 1;
    assert!(matches!(
        rejected(evaluate(&project, &changed)),
        Revision3DialogLineInsertConflictV1::ProjectRevisionConflict { .. }
    ));

    changed = request.clone();
    changed.expected_target = target(0x99);
    assert_eq!(
        rejected(evaluate(&project, &changed)),
        Revision3DialogLineInsertConflictV1::ProjectTargetMismatch
    );

    let mut overflow_project = project;
    overflow_project.revision = u64::MAX;
    changed = request;
    changed.expected_revision = u64::MAX;
    assert_eq!(
        rejected(evaluate(&overflow_project, &changed)),
        Revision3DialogLineInsertConflictV1::ProjectRevisionOverflow
    );
}

#[test]
fn zero_shared_colliding_and_casefolded_identities_fail_closed() {
    let project = basis();
    let baseline = create_request(true);

    let mut changed = baseline.clone();
    changed.line_id = EntityId::from_bytes([0; 16]);
    assert_eq!(
        rejected(evaluate(&project, &changed)),
        Revision3DialogLineInsertConflictV1::ZeroEntityId {
            role: Revision3DialogEntityRoleV1::DialogLine,
        }
    );

    changed = baseline.clone();
    changed.line_id = id(2);
    assert_eq!(
        rejected(evaluate(&project, &changed)),
        Revision3DialogLineInsertConflictV1::SharedEntityId
    );

    let mut collision_project = project.clone();
    collision_project.entities.insert(
        id(1),
        Entity {
            id: id(1),
            display_name: "Prior".to_owned(),
            origin: OriginRef::Imported {
                importer: "tests".to_owned(),
                source_seal: seal(0x66, 1),
                external_identity: None,
            },
            revision: 0,
            payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                loc_id: "PRIOR".to_owned(),
                texts: BTreeMap::new(),
            }),
        },
    );
    assert!(matches!(
        rejected(evaluate(&collision_project, &baseline)),
        Revision3DialogLineInsertConflictV1::EntityIdCollision {
            role: Revision3DialogEntityRoleV1::DialogLine,
            entity,
        } if entity == id(1)
    ));

    let mut duplicate_project = project;
    duplicate_project.entities.insert(
        id(9),
        Entity {
            id: id(9),
            display_name: "Prior localization".to_owned(),
            origin: OriginRef::Imported {
                importer: "tests".to_owned(),
                source_seal: seal(0x67, 1),
                external_identity: None,
            },
            revision: 0,
            payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                loc_id: "gore_asghan_welcome".to_owned(),
                texts: BTreeMap::from([(locale("de"), "Alt".to_owned())]),
            }),
        },
    );
    assert!(matches!(
        rejected(evaluate(&duplicate_project, &baseline)),
        Revision3DialogLineInsertConflictV1::DuplicateLocalizationIdentity {
            existing_entity,
            ..
        } if existing_entity == id(9)
    ));
}

#[test]
fn invalid_user_values_localization_texts_and_slot_locale_return_no_candidate() {
    let project = basis();
    let baseline = create_request(true);

    let mut changed = baseline.clone();
    changed.line_display_name = " line ".to_owned();
    assert_eq!(
        rejected(evaluate(&project, &changed)),
        Revision3DialogLineInsertConflictV1::InvalidLineDisplayName
    );

    changed = baseline.clone();
    changed.line_authored_identity = "not canonical".to_owned();
    assert_eq!(
        rejected(evaluate(&project, &changed)),
        Revision3DialogLineInsertConflictV1::InvalidLineAuthoredIdentity
    );

    changed = baseline.clone();
    changed.speaker_hint = Some(" Asghan".to_owned());
    assert_eq!(
        rejected(evaluate(&project, &changed)),
        Revision3DialogLineInsertConflictV1::InvalidSpeakerHint
    );

    changed = baseline.clone();
    let Revision3DialogLocalizationIntentV1::Create { loc_id, .. } = &mut changed.localization
    else {
        unreachable!()
    };
    *loc_id = "unsafe/name".to_owned();
    assert_eq!(
        rejected(evaluate(&project, &changed)),
        Revision3DialogLineInsertConflictV1::InvalidLocalizationId
    );

    changed = baseline.clone();
    let Revision3DialogLocalizationIntentV1::Create { texts, .. } = &mut changed.localization
    else {
        unreachable!()
    };
    texts.insert(locale("de"), "   ".to_owned());
    assert_eq!(
        rejected(evaluate(&project, &changed)),
        Revision3DialogLineInsertConflictV1::InvalidLocalizationTexts
    );

    changed = baseline;
    changed.voice_slot.as_mut().unwrap().locale = locale("fr");
    assert_eq!(
        rejected(evaluate(&project, &changed)),
        Revision3DialogLineInsertConflictV1::VoiceSlotLocaleHasNoText {
            locale: locale("fr"),
        }
    );
}

#[test]
fn reuse_requires_exact_kind_revision_identity_and_unambiguous_loc_id() {
    let mut project = basis();
    project.entities.insert(
        id(2),
        Entity {
            id: id(2),
            display_name: "Existing".to_owned(),
            origin: OriginRef::Imported {
                importer: "tests".to_owned(),
                source_seal: seal(0x70, 3),
                external_identity: None,
            },
            revision: 4,
            payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                loc_id: "GORE_EXISTING".to_owned(),
                texts: BTreeMap::from([(locale("de"), "Vorhanden".to_owned())]),
            }),
        },
    );
    let baseline = Revision3DialogLineInsertRequestV1 {
        expected_head: head(0x30),
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        expected_target: project.target.clone(),
        line_id: id(1),
        line_display_name: "Reuse".to_owned(),
        line_authored_identity: "DIA_GORE_REUSE".to_owned(),
        speaker_hint: None,
        localization: Revision3DialogLocalizationIntentV1::ReuseExact {
            localization_id: id(2),
            expected_localization_revision: 4,
            expected_loc_id: "GORE_EXISTING".to_owned(),
        },
        voice_slot: None,
    };

    let mut changed = baseline.clone();
    let Revision3DialogLocalizationIntentV1::ReuseExact {
        expected_localization_revision,
        ..
    } = &mut changed.localization
    else {
        unreachable!()
    };
    *expected_localization_revision = 3;
    assert!(matches!(
        rejected(evaluate(&project, &changed)),
        Revision3DialogLineInsertConflictV1::LocalizationRevisionConflict { .. }
    ));

    changed = baseline.clone();
    let Revision3DialogLocalizationIntentV1::ReuseExact {
        expected_loc_id, ..
    } = &mut changed.localization
    else {
        unreachable!()
    };
    *expected_loc_id = "GORE_OTHER".to_owned();
    assert!(matches!(
        rejected(evaluate(&project, &changed)),
        Revision3DialogLineInsertConflictV1::LocalizationIdentityConflict { .. }
    ));

    changed = baseline.clone();
    let Revision3DialogLocalizationIntentV1::ReuseExact {
        localization_id, ..
    } = &mut changed.localization
    else {
        unreachable!()
    };
    *localization_id = id(8);
    assert_eq!(
        rejected(evaluate(&project, &changed)),
        Revision3DialogLineInsertConflictV1::LocalizationMissingOrWrongKind {
            localization: id(8),
        }
    );

    project.entities.insert(
        id(7),
        Entity {
            id: id(7),
            display_name: "Existing owner".to_owned(),
            origin: OriginRef::Imported {
                importer: "tests".to_owned(),
                source_seal: seal(0x72, 4),
                external_identity: None,
            },
            revision: 0,
            payload: EntityPayload::DialogLine(DialogLine {
                localization: TypedRef::new(
                    project.project_id,
                    id(2),
                    EntityKind::LocalizationEntry,
                ),
                speaker_hint: None,
                voice_slots: BTreeMap::new(),
            }),
        },
    );
    assert_eq!(
        rejected(evaluate(&project, &baseline)),
        Revision3DialogLineInsertConflictV1::LocalizationAlreadyReferenced {
            localization: id(2),
            owner_line: id(7),
        }
    );
    project.entities.remove(&id(7));

    project.entities.insert(
        id(9),
        Entity {
            id: id(9),
            display_name: "Duplicate".to_owned(),
            origin: OriginRef::Imported {
                importer: "tests".to_owned(),
                source_seal: seal(0x71, 3),
                external_identity: None,
            },
            revision: 0,
            payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                loc_id: "gore_existing".to_owned(),
                texts: BTreeMap::from([(locale("de"), "Doppelt".to_owned())]),
            }),
        },
    );
    assert!(matches!(
        rejected(evaluate(&project, &baseline)),
        Revision3DialogLineInsertConflictV1::DuplicateLocalizationIdentity {
            existing_entity,
            ..
        } if existing_entity == id(9)
    ));
}

#[test]
fn candidate_project_size_pressure_is_distinct_from_invalid_model() {
    let mut project = basis();
    let initial_len = project.to_canonical_json().unwrap().len();
    let remaining_budget = 64usize;
    project
        .meta
        .name
        .push_str(&"x".repeat(MAX_PROJECT_JSON_BYTES - initial_len - remaining_budget));
    let canonical_basis = project.to_canonical_json().unwrap();
    assert_eq!(
        canonical_basis.len(),
        MAX_PROJECT_JSON_BYTES - remaining_budget
    );
    let request = create_request(false).to_canonical_json().unwrap();

    let conflict = rejected(
        apply_revision3_dialog_line_insert_transaction_v1(&head(0x30), &canonical_basis, &request)
            .unwrap(),
    );
    assert!(matches!(
        conflict,
        Revision3DialogLineInsertConflictV1::CandidateTooLarge { actual, limit }
            if actual > limit && limit == MAX_PROJECT_JSON_BYTES
    ));
}
