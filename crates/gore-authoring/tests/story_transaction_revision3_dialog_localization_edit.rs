use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::model_revision3::{
    DialogLine, Entity, EntityKind, EntityPayload, LocalizationEntry, OggCodec, OggMetadata,
    OriginRef, SchemaRevisionV3, TypedRef, VoiceSlot, VoiceTake, VoiceTakeStatus,
    VoiceTargetResolution,
};
use gore_authoring::{
    apply_revision3_dialog_localization_edit_transaction_v1, AssetMeta, AssetRef, AssetStoreIndex,
    ContentSeal, EntityId, FormatV2, GameGenerationAnchor, LocaleCode, ProjectId, ProjectMeta,
    ProjectRevision3, Revision3DialogLocalizationEditBuildStatusV1,
    Revision3DialogLocalizationEditConflictV1, Revision3DialogLocalizationEditEvaluationV1,
    Revision3DialogLocalizationEditOutcomeV1, Revision3DialogLocalizationEditPublicationStatusV1,
    Revision3DialogLocalizationEditRequestJsonErrorV1, Revision3DialogLocalizationEditRequestV1,
    Revision3DialogLocalizationEditRuntimeStatusV1,
    Revision3DialogLocalizationEditTopicAuthorityV1, Sha256Digest, WorkingHead, WorkingStoreFormat,
    MAX_PROJECT_JSON_BYTES, MAX_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_JSON_BYTES_V1,
    MAX_REVISION3_DIALOG_LOCALIZATION_TEXTS_V1, MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_BYTES_V1,
    MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_TOTAL_BYTES_V1, REVISION3_VOICE_SLOT_GENERATOR_ID_V1,
    REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1,
};

const LOCALIZATION_ID: u8 = 1;
const LOC_ID: &str = "GORE_ASGHAN_WELCOME";

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

fn imported_origin(tag: u8) -> OriginRef {
    OriginRef::Imported {
        importer: "localization-edit-tests".to_owned(),
        source_seal: seal(tag, 123),
        external_identity: None,
    }
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
    let localization = Entity {
        id: id(LOCALIZATION_ID),
        display_name: "Asghan welcome text".to_owned(),
        origin: new_origin(LOC_ID),
        revision: 4,
        payload: EntityPayload::LocalizationEntry(LocalizationEntry {
            loc_id: LOC_ID.to_owned(),
            texts: BTreeMap::from([
                (locale("de"), "Willkommen im Alten Lager.".to_owned()),
                (locale("en"), "Welcome to the Old Camp.".to_owned()),
            ]),
        }),
    };
    let unrelated = Entity {
        id: id(9),
        display_name: "Unrelated text".to_owned(),
        origin: new_origin("GORE_UNRELATED"),
        revision: 8,
        payload: EntityPayload::LocalizationEntry(LocalizationEntry {
            loc_id: "GORE_UNRELATED".to_owned(),
            texts: BTreeMap::from([(locale("de"), "Unberuehrt.".to_owned())]),
        }),
    };
    ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id: project_id(0x10),
        revision: 7,
        meta: ProjectMeta {
            name: "Localization edit".to_owned(),
            version: "1.0.0".to_owned(),
            author: "tests".to_owned(),
        },
        target: target(0x20),
        authoring_locales: BTreeSet::from([locale("de"), locale("en")]),
        entities: BTreeMap::from([(id(LOCALIZATION_ID), localization), (id(9), unrelated)]),
        asset_store: assets,
    }
}

fn request(project: &ProjectRevision3) -> Revision3DialogLocalizationEditRequestV1 {
    Revision3DialogLocalizationEditRequestV1 {
        expected_head: head(0x30),
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        expected_target: project.target.clone(),
        localization_id: id(LOCALIZATION_ID),
        expected_localization_revision: project.entities[&id(LOCALIZATION_ID)].revision,
        expected_loc_id: LOC_ID.to_owned(),
        texts: BTreeMap::from([
            (locale("de"), "Geaenderter Text.".to_owned()),
            (locale("en"), "Changed text.".to_owned()),
        ]),
    }
}

fn evaluate(
    project: &ProjectRevision3,
    request: &Revision3DialogLocalizationEditRequestV1,
) -> Revision3DialogLocalizationEditEvaluationV1 {
    apply_revision3_dialog_localization_edit_transaction_v1(
        &head(0x30),
        &project.to_canonical_json().unwrap(),
        &request.to_canonical_json().unwrap(),
    )
    .unwrap()
}

fn applied(
    value: Revision3DialogLocalizationEditEvaluationV1,
) -> Revision3DialogLocalizationEditOutcomeV1 {
    match value {
        Revision3DialogLocalizationEditEvaluationV1::Applied(outcome) => *outcome,
        Revision3DialogLocalizationEditEvaluationV1::Rejected(rejection) => {
            panic!("unexpected rejection: {}", rejection.conflict)
        }
    }
}

fn rejected(
    value: Revision3DialogLocalizationEditEvaluationV1,
) -> Revision3DialogLocalizationEditConflictV1 {
    match value {
        Revision3DialogLocalizationEditEvaluationV1::Rejected(rejection) => rejection.conflict,
        Revision3DialogLocalizationEditEvaluationV1::Applied(_) => panic!("unexpected candidate"),
    }
}

fn add_line(project: &mut ProjectRevision3, line_tag: u8, slot: Option<(u8, bool)>) {
    let project_id = project.project_id;
    let mut voice_slots = BTreeMap::new();
    if let Some((slot_tag, with_candidate)) = slot {
        voice_slots.insert(
            locale("de"),
            TypedRef::new(project_id, id(slot_tag), EntityKind::VoiceSlot),
        );
        let candidates = if with_candidate {
            let take_tag = slot_tag + 1;
            let asset = AssetRef {
                sha256: digest(take_tag),
                byte_len: 1024,
                logical_name: format!("take-{take_tag}.ogg"),
            };
            project.asset_store.assets.insert(
                asset.sha256,
                AssetMeta {
                    byte_len: asset.byte_len,
                    media_type: "audio/ogg".to_owned(),
                },
            );
            project.entities.insert(
                id(take_tag),
                Entity {
                    id: id(take_tag),
                    display_name: format!("Take {take_tag}"),
                    origin: imported_origin(take_tag),
                    revision: 1,
                    payload: EntityPayload::VoiceTake(VoiceTake {
                        locale: locale("de"),
                        asset,
                        ogg: OggMetadata {
                            codec: OggCodec::Vorbis,
                            channels: 1,
                            sample_rate: 48_000,
                            pages: 3,
                            logical_streams: 1,
                        },
                        status: VoiceTakeStatus::Recorded,
                    }),
                },
            );
            vec![TypedRef::new(
                project_id,
                id(take_tag),
                EntityKind::VoiceTake,
            )]
        } else {
            Vec::new()
        };
        project.entities.insert(
            id(slot_tag),
            Entity {
                id: id(slot_tag),
                display_name: format!("German slot {slot_tag}"),
                origin: OriginRef::Generated {
                    generator_id: REVISION3_VOICE_SLOT_GENERATOR_ID_V1.to_owned(),
                    generator_version: REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1,
                    owner: TypedRef::new(project_id, id(line_tag), EntityKind::DialogLine),
                },
                revision: 2,
                payload: EntityPayload::VoiceSlot(VoiceSlot {
                    locale: locale("de"),
                    target_resolution: VoiceTargetResolution::Unresolved,
                    candidates,
                    selected: None,
                }),
            },
        );
    }
    project.entities.insert(
        id(line_tag),
        Entity {
            id: id(line_tag),
            display_name: format!("Dialog line {line_tag}"),
            origin: new_origin(&format!("DIA_GORE_{line_tag}")),
            revision: 3,
            payload: EntityPayload::DialogLine(DialogLine {
                localization: TypedRef::new(
                    project_id,
                    id(LOCALIZATION_ID),
                    EntityKind::LocalizationEntry,
                ),
                speaker_hint: Some("Asghan".to_owned()),
                voice_slots,
            }),
        },
    );
}

#[test]
fn exact_full_multibyte_text_edit_reopens_and_confines_the_delta() {
    let mut project = basis();
    add_line(&mut project, 2, None);
    add_line(&mut project, 3, None);
    let before = project.clone();
    let long_text = "Grueße 世界 🙂 ".repeat(90);
    assert!(long_text.len() > 512);
    let mut request = request(&project);
    request.texts = BTreeMap::from([
        (locale("de"), long_text.clone()),
        (locale("fr"), "Bienvenue dans l'ancien camp.".to_owned()),
    ]);

    let first = applied(evaluate(&project, &request));
    let second = applied(evaluate(&project, &request));
    assert_eq!(first, second);
    assert_eq!(first.project.revision, before.revision + 1);
    assert_eq!(first.basis_head, head(0x30));
    assert_eq!(first.localization_id, id(LOCALIZATION_ID));
    assert_eq!(first.added_locales, BTreeSet::from([locale("fr")]));
    assert_eq!(first.removed_locales, BTreeSet::from([locale("en")]));
    assert_eq!(
        first.build_status,
        Revision3DialogLocalizationEditBuildStatusV1::Blocked
    );
    assert_eq!(
        first.runtime_status,
        Revision3DialogLocalizationEditRuntimeStatusV1::RuntimeUnqualified
    );
    assert_eq!(
        first.topic_authority,
        Revision3DialogLocalizationEditTopicAuthorityV1::NotGranted
    );
    assert_eq!(
        first.publication_status,
        Revision3DialogLocalizationEditPublicationStatusV1::NotSupported
    );
    assert_eq!(
        ProjectRevision3::from_json(&first.canonical_project_json).unwrap(),
        first.project
    );

    assert_eq!(first.project.meta, before.meta);
    assert_eq!(first.project.target, before.target);
    assert_eq!(first.project.asset_store, before.asset_store);
    assert_eq!(
        first.project.entities.keys().collect::<Vec<_>>(),
        before.entities.keys().collect::<Vec<_>>()
    );
    for entity_id in before
        .entities
        .keys()
        .copied()
        .filter(|value| *value != id(LOCALIZATION_ID))
    {
        assert_eq!(
            first.project.entities[&entity_id],
            before.entities[&entity_id]
        );
    }
    let before_target = &before.entities[&id(LOCALIZATION_ID)];
    let after_target = &first.project.entities[&id(LOCALIZATION_ID)];
    assert_eq!(after_target.id, before_target.id);
    assert_eq!(after_target.display_name, before_target.display_name);
    assert_eq!(after_target.origin, before_target.origin);
    assert_eq!(after_target.revision, before_target.revision + 1);
    let EntityPayload::LocalizationEntry(after_localization) = &after_target.payload else {
        panic!("expected localization")
    };
    assert_eq!(after_localization.loc_id, LOC_ID);
    assert_eq!(after_localization.texts[&locale("de")], long_text);
    assert_eq!(after_localization.texts, request.texts);
    assert_eq!(
        first.project.authoring_locales,
        BTreeSet::from([locale("de"), locale("en"), locale("fr")])
    );
    assert!(first.project.validate_closed_model().is_ok());
}

#[test]
fn removed_locale_stays_in_global_authoring_locales_and_added_locale_is_inserted() {
    let project = basis();
    let mut request = request(&project);
    request.texts = BTreeMap::from([
        (locale("de"), "Neu.".to_owned()),
        (locale("fr"), "Nouveau.".to_owned()),
    ]);
    let outcome = applied(evaluate(&project, &request));
    assert!(outcome.project.authoring_locales.contains(&locale("en")));
    assert!(outcome.project.authoring_locales.contains(&locale("fr")));
    assert_eq!(outcome.added_locales, BTreeSet::from([locale("fr")]));
    assert_eq!(outcome.removed_locales, BTreeSet::from([locale("en")]));
}

#[test]
fn empty_voice_slot_requires_nonblank_locale_but_allows_a_nonblank_edit() {
    let mut project = basis();
    add_line(&mut project, 2, Some((3, false)));

    let mut removed = request(&project);
    removed.texts = BTreeMap::from([(locale("en"), "Still here.".to_owned())]);
    assert!(matches!(
        rejected(evaluate(&project, &removed)),
        Revision3DialogLocalizationEditConflictV1::VoiceSlotLocaleRemovedOrBlank {
            line,
            slot,
            locale: protected
        } if line == id(2) && slot == id(3) && protected == locale("de")
    ));

    let mut blank = request(&project);
    blank.texts.insert(locale("de"), " \t ".to_owned());
    assert!(matches!(
        rejected(evaluate(&project, &blank)),
        Revision3DialogLocalizationEditConflictV1::VoiceSlotLocaleRemovedOrBlank { .. }
    ));

    let mut changed = request(&project);
    changed
        .texts
        .insert(locale("de"), "Nicht stehen bleiben.".to_owned());
    let outcome = applied(evaluate(&project, &changed));
    let EntityPayload::LocalizationEntry(localization) =
        &outcome.project.entities[&id(LOCALIZATION_ID)].payload
    else {
        panic!("expected localization")
    };
    assert_eq!(localization.texts[&locale("de")], "Nicht stehen bleiben.");
}

#[test]
fn slot_with_candidates_protects_its_text_but_not_unrelated_locales() {
    let mut project = basis();
    add_line(&mut project, 2, Some((3, true)));

    let mut protected = request(&project);
    protected
        .texts
        .insert(locale("de"), "A different spoken line.".to_owned());
    assert!(matches!(
        rejected(evaluate(&project, &protected)),
        Revision3DialogLocalizationEditConflictV1::VoiceSlotCandidatesProtectText {
            line,
            slot,
            locale: protected_locale
        } if line == id(2) && slot == id(3) && protected_locale == locale("de")
    ));

    let mut unrelated = request(&project);
    let original_de = match &project.entities[&id(LOCALIZATION_ID)].payload {
        EntityPayload::LocalizationEntry(value) => value.texts[&locale("de")].clone(),
        _ => unreachable!(),
    };
    unrelated.texts.insert(locale("de"), original_de);
    unrelated
        .texts
        .insert(locale("en"), "A changed subtitle.".to_owned());
    let outcome = applied(evaluate(&project, &unrelated));
    assert_eq!(outcome.project.entities[&id(2)], project.entities[&id(2)]);
    assert_eq!(outcome.project.entities[&id(3)], project.entities[&id(3)]);
    assert_eq!(outcome.project.entities[&id(4)], project.entities[&id(4)]);
}

#[test]
fn only_new_localizations_are_editable() {
    for origin in [
        imported_origin(0x40),
        OriginRef::Vanilla {
            generation: target(0x20),
            catalog_layer: "loc".to_owned(),
            canonical_selector: LOC_ID.to_owned(),
            source_seal: seal(0x41, 77),
        },
    ] {
        let mut project = basis();
        project
            .entities
            .get_mut(&id(LOCALIZATION_ID))
            .unwrap()
            .origin = origin;
        assert!(matches!(
            rejected(evaluate(&project, &request(&project))),
            Revision3DialogLocalizationEditConflictV1::LocalizationOriginNotNew {
                localization
            } if localization == id(LOCALIZATION_ID)
        ));
    }
}

#[test]
fn exact_head_project_target_entity_revision_and_loc_id_are_bound() {
    let project = basis();

    let result = apply_revision3_dialog_localization_edit_transaction_v1(
        &head(0x31),
        &project.to_canonical_json().unwrap(),
        &request(&project).to_canonical_json().unwrap(),
    )
    .unwrap();
    assert_eq!(
        rejected(result),
        Revision3DialogLocalizationEditConflictV1::CurrentHeadMismatch
    );

    let mut wrong_project = request(&project);
    wrong_project.expected_project_id = project_id(0x11);
    assert!(matches!(
        rejected(evaluate(&project, &wrong_project)),
        Revision3DialogLocalizationEditConflictV1::ProjectIdentityMismatch { .. }
    ));
    let mut stale_project = request(&project);
    stale_project.expected_revision -= 1;
    assert!(matches!(
        rejected(evaluate(&project, &stale_project)),
        Revision3DialogLocalizationEditConflictV1::ProjectRevisionConflict { .. }
    ));
    let mut wrong_target = request(&project);
    wrong_target.expected_target = target(0x21);
    assert_eq!(
        rejected(evaluate(&project, &wrong_target)),
        Revision3DialogLocalizationEditConflictV1::ProjectTargetMismatch
    );
    let mut missing = request(&project);
    missing.localization_id = id(8);
    assert!(matches!(
        rejected(evaluate(&project, &missing)),
        Revision3DialogLocalizationEditConflictV1::LocalizationMissingOrWrongKind { .. }
    ));
    let mut stale_entity = request(&project);
    stale_entity.expected_localization_revision -= 1;
    assert!(matches!(
        rejected(evaluate(&project, &stale_entity)),
        Revision3DialogLocalizationEditConflictV1::LocalizationRevisionConflict { .. }
    ));
    let mut wrong_loc_id = request(&project);
    wrong_loc_id.expected_loc_id = "GORE_OTHER".to_owned();
    assert!(matches!(
        rejected(evaluate(&project, &wrong_loc_id)),
        Revision3DialogLocalizationEditConflictV1::LocalizationIdentityConflict { .. }
    ));
}

#[test]
fn noop_is_rejected_without_exposing_a_candidate() {
    let project = basis();
    let mut request = request(&project);
    request.texts = match &project.entities[&id(LOCALIZATION_ID)].payload {
        EntityPayload::LocalizationEntry(value) => value.texts.clone(),
        _ => unreachable!(),
    };
    assert_eq!(
        rejected(evaluate(&project, &request)),
        Revision3DialogLocalizationEditConflictV1::NoChanges
    );
}

#[test]
fn project_and_localization_revision_overflow_are_rejected() {
    let mut project = basis();
    project.revision = u64::MAX;
    let mut project_overflow_request = request(&project);
    project_overflow_request.expected_revision = u64::MAX;
    assert_eq!(
        rejected(evaluate(&project, &project_overflow_request)),
        Revision3DialogLocalizationEditConflictV1::ProjectRevisionOverflow
    );

    let mut project = basis();
    project
        .entities
        .get_mut(&id(LOCALIZATION_ID))
        .unwrap()
        .revision = u64::MAX;
    let request = request(&project);
    assert!(matches!(
        rejected(evaluate(&project, &request)),
        Revision3DialogLocalizationEditConflictV1::LocalizationRevisionOverflow {
            localization
        } if localization == id(LOCALIZATION_ID)
    ));
}

#[test]
fn replacement_text_capacity_nul_and_nonblank_rules_are_closed() {
    let project = basis();

    for texts in [
        BTreeMap::new(),
        BTreeMap::from([(locale("de"), "  \n ".to_owned())]),
        BTreeMap::from([(locale("de"), "bad\0text".to_owned())]),
        BTreeMap::from([(
            locale("de"),
            "x".repeat(MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_BYTES_V1 + 1),
        )]),
    ] {
        let mut request = request(&project);
        request.texts = texts;
        assert_eq!(
            rejected(evaluate(&project, &request)),
            Revision3DialogLocalizationEditConflictV1::InvalidLocalizationTexts
        );
    }

    let too_many = (0..=MAX_REVISION3_DIALOG_LOCALIZATION_TEXTS_V1)
        .map(|index| (locale(&format!("en-x{index}")), format!("text {index}")))
        .collect::<BTreeMap<_, _>>();
    let mut too_many_request = request(&project);
    too_many_request.texts = too_many;
    assert_eq!(
        rejected(evaluate(&project, &too_many_request)),
        Revision3DialogLocalizationEditConflictV1::InvalidLocalizationTexts
    );

    let total_too_large = (0..9)
        .map(|index| {
            (
                locale(&format!("en-y{index}")),
                "x".repeat(MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_TOTAL_BYTES_V1 / 9 + 1),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert!(total_too_large
        .values()
        .all(|text| { text.len() <= MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_BYTES_V1 }));
    let mut request = request(&project);
    request.texts = total_too_large;
    assert_eq!(
        rejected(evaluate(&project, &request)),
        Revision3DialogLocalizationEditConflictV1::InvalidLocalizationTexts
    );
}

#[test]
fn candidate_project_capacity_is_distinct_from_a_valid_exact_basis() {
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
    let mut request = request(&project);
    request.texts = BTreeMap::from([
        (locale("de"), "d".repeat(256)),
        (locale("en"), "e".repeat(256)),
    ]);
    let conflict = rejected(
        apply_revision3_dialog_localization_edit_transaction_v1(
            &head(0x30),
            &canonical_basis,
            &request.to_canonical_json().unwrap(),
        )
        .unwrap(),
    );
    assert!(matches!(
        conflict,
        Revision3DialogLocalizationEditConflictV1::CandidateTooLarge {
            actual,
            limit: MAX_PROJECT_JSON_BYTES,
        } if actual > MAX_PROJECT_JSON_BYTES
    ));
}

#[test]
fn request_wire_is_bounded_canonical_duplicate_safe_and_locale_canonical() {
    let project = basis();
    let request = request(&project);
    let canonical = request.to_canonical_json().unwrap();
    assert_eq!(
        Revision3DialogLocalizationEditRequestV1::from_json(&canonical).unwrap(),
        request
    );
    assert!(matches!(
        Revision3DialogLocalizationEditRequestV1::from_json(&(canonical.clone() + "\n")),
        Err(Revision3DialogLocalizationEditRequestJsonErrorV1::NonCanonicalJson)
    ));
    let duplicate = canonical.replacen(
        "\"expected_revision\":7",
        "\"expected_revision\":7,\"expected_revision\":7",
        1,
    );
    assert!(matches!(
        Revision3DialogLocalizationEditRequestV1::from_json(&duplicate),
        Err(Revision3DialogLocalizationEditRequestJsonErrorV1::InvalidJson(_))
    ));
    let noncanonical_locale = canonical.replacen("\"de\":", "\"DE\":", 1);
    assert!(matches!(
        Revision3DialogLocalizationEditRequestV1::from_json(&noncanonical_locale),
        Err(Revision3DialogLocalizationEditRequestJsonErrorV1::InvalidJson(_))
    ));
    let oversized = "x".repeat(MAX_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_JSON_BYTES_V1 + 1);
    assert!(matches!(
        Revision3DialogLocalizationEditRequestV1::from_json(&oversized),
        Err(Revision3DialogLocalizationEditRequestJsonErrorV1::InputTooLarge { .. })
    ));
}
