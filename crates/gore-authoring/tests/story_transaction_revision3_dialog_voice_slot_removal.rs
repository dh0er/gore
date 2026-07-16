use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::model_revision3::{
    DialogLine, Entity, EntityKind, EntityPayload, LocalizationEntry, OggCodec, OggMetadata,
    OriginRef, SchemaRevisionV3, TypedRef, VoiceMemberProof, VoiceOperation, VoiceSlot, VoiceTake,
    VoiceTakeStatus, VoiceTarget, VoiceTargetResolution,
};
use gore_authoring::{
    apply_revision3_dialog_voice_slot_removal_transaction_v1, ArchiveSeal, AssetMeta, AssetRef,
    AssetStoreIndex, ContentSeal, EntityId, FormatV2, GameGenerationAnchor, LocaleCode, ProjectId,
    ProjectMeta, ProjectRevision3, Revision3ContentReferenceRoleV1,
    Revision3DialogVoiceSlotRemovalBuildStatusV1, Revision3DialogVoiceSlotRemovalConflictV1,
    Revision3DialogVoiceSlotRemovalErrorV1, Revision3DialogVoiceSlotRemovalEvaluationV1,
    Revision3DialogVoiceSlotRemovalOutcomeV1, Revision3DialogVoiceSlotRemovalPublicationStatusV1,
    Revision3DialogVoiceSlotRemovalRequestJsonErrorV1, Revision3DialogVoiceSlotRemovalRequestV1,
    Revision3DialogVoiceSlotRemovalRuntimeStatusV1,
    Revision3DialogVoiceSlotRemovalTargetAuthorityV1,
    Revision3DialogVoiceSlotRemovalTargetResolutionV1, Sha256Digest, WorkingHead,
    WorkingStoreFormat, MAX_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_JSON_BYTES_V1,
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

fn target() -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: seal(0x20, 171_698_176),
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

fn voice_target(archive: &str, member: &str, tag: u8) -> VoiceTarget {
    VoiceTarget {
        archive: archive.to_owned(),
        member: member.to_owned(),
        operation: VoiceOperation::Replace,
        archive_seal: ArchiveSeal {
            byte_len: 4096,
            sha256: digest(tag),
        },
        member_proof: VoiceMemberProof::Present {
            uncompressed_size: 1024,
            crc32: u32::from(tag),
        },
    }
}

fn basis(resolution: VoiceTargetResolution) -> ProjectRevision3 {
    let localization_id = id(2);
    let line_id = id(3);
    let slot_id = id(4);
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
        project_id: project_id(0x10),
        revision: 7,
        meta: ProjectMeta {
            name: "Dialog VoiceSlot removal".to_owned(),
            version: "1.0.0".to_owned(),
            author: "tests".to_owned(),
        },
        target: target(),
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
                            (de.clone(), "Willkommen.".to_owned()),
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
                            project_id(0x10),
                            localization_id,
                            EntityKind::LocalizationEntry,
                        ),
                        speaker_hint: Some("Asghan".to_owned()),
                        voice_slots: BTreeMap::from([(
                            de.clone(),
                            TypedRef::new(project_id(0x10), slot_id, EntityKind::VoiceSlot),
                        )]),
                    }),
                },
            ),
            (
                slot_id,
                Entity {
                    id: slot_id,
                    display_name: "Asghan German voice".to_owned(),
                    origin: OriginRef::Generated {
                        generator_id: REVISION3_VOICE_SLOT_GENERATOR_ID_V1.to_owned(),
                        generator_version: REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1,
                        owner: TypedRef::new(project_id(0x10), line_id, EntityKind::DialogLine),
                    },
                    revision: 4,
                    payload: EntityPayload::VoiceSlot(VoiceSlot {
                        locale: de,
                        target_resolution: resolution,
                        candidates: Vec::new(),
                        selected: None,
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

fn request() -> Revision3DialogVoiceSlotRemovalRequestV1 {
    Revision3DialogVoiceSlotRemovalRequestV1 {
        expected_head: head(0x31),
        expected_project_id: project_id(0x10),
        expected_revision: 7,
        expected_target: target(),
        line_id: id(3),
        expected_line_revision: 3,
        localization_id: id(2),
        expected_loc_id: "GORE_ASGHAN_GREETING".to_owned(),
        locale: locale("de"),
        slot_id: id(4),
        expected_slot_revision: 4,
    }
}

fn evaluate_with_head(
    project: &ProjectRevision3,
    request: &Revision3DialogVoiceSlotRemovalRequestV1,
    exact_head: &WorkingHead,
) -> Result<Revision3DialogVoiceSlotRemovalEvaluationV1, Revision3DialogVoiceSlotRemovalErrorV1> {
    apply_revision3_dialog_voice_slot_removal_transaction_v1(
        exact_head,
        &project.to_canonical_json().unwrap(),
        &request.to_canonical_json().unwrap(),
    )
}

fn evaluate(
    project: &ProjectRevision3,
    request: &Revision3DialogVoiceSlotRemovalRequestV1,
) -> Revision3DialogVoiceSlotRemovalEvaluationV1 {
    evaluate_with_head(project, request, &head(0x31)).unwrap()
}

fn applied(
    evaluation: Revision3DialogVoiceSlotRemovalEvaluationV1,
) -> Revision3DialogVoiceSlotRemovalOutcomeV1 {
    match evaluation {
        Revision3DialogVoiceSlotRemovalEvaluationV1::Applied(outcome) => *outcome,
        Revision3DialogVoiceSlotRemovalEvaluationV1::Rejected(rejection) => {
            panic!("unexpected rejection: {}", rejection.conflict)
        }
    }
}

fn conflict(
    evaluation: Revision3DialogVoiceSlotRemovalEvaluationV1,
) -> Revision3DialogVoiceSlotRemovalConflictV1 {
    match evaluation {
        Revision3DialogVoiceSlotRemovalEvaluationV1::Rejected(rejection) => rejection.conflict,
        Revision3DialogVoiceSlotRemovalEvaluationV1::Applied(_) => panic!("unexpected candidate"),
    }
}

fn line(project: &ProjectRevision3) -> &DialogLine {
    let EntityPayload::DialogLine(line) = &project.entities[&id(3)].payload else {
        panic!("expected DialogLine")
    };
    line
}

fn slot_mut(project: &mut ProjectRevision3) -> &mut VoiceSlot {
    let EntityPayload::VoiceSlot(slot) = &mut project.entities.get_mut(&id(4)).unwrap().payload
    else {
        panic!("expected VoiceSlot")
    };
    slot
}

#[test]
fn exact_empty_generated_slot_and_line_edge_are_removed_atomically() {
    let project = basis(VoiceTargetResolution::Unresolved);
    let original = project.clone();
    let outcome = applied(evaluate(&project, &request()));

    assert_eq!(
        project, original,
        "the pure transaction must not mutate input"
    );
    assert_eq!(outcome.project.revision, 8);
    assert_eq!(outcome.line_id, id(3));
    assert_eq!(outcome.line_revision, 4);
    assert_eq!(outcome.localization_id, id(2));
    assert_eq!(outcome.slot_id, id(4));
    assert_eq!(outcome.removed_slot_revision, 4);
    assert_eq!(outcome.locale, locale("de"));
    assert_eq!(outcome.loc_id, "GORE_ASGHAN_GREETING");
    assert_eq!(
        outcome.removed_target_resolution,
        Revision3DialogVoiceSlotRemovalTargetResolutionV1::Unresolved
    );
    assert_eq!(
        outcome.build_status,
        Revision3DialogVoiceSlotRemovalBuildStatusV1::Blocked
    );
    assert_eq!(
        outcome.runtime_status,
        Revision3DialogVoiceSlotRemovalRuntimeStatusV1::RuntimeUnqualified
    );
    assert_eq!(
        outcome.target_authority,
        Revision3DialogVoiceSlotRemovalTargetAuthorityV1::NotGranted
    );
    assert_eq!(
        outcome.publication_status,
        Revision3DialogVoiceSlotRemovalPublicationStatusV1::NotSupported
    );
    assert_eq!(outcome.basis_head, head(0x31));
    assert!(line(&outcome.project).voice_slots.is_empty());
    assert_eq!(outcome.project.entities[&id(3)].revision, 4);
    assert!(!outcome.project.entities.contains_key(&id(4)));
    assert_eq!(outcome.project.entities[&id(2)], original.entities[&id(2)]);
    assert_eq!(outcome.project.entities[&id(5)], original.entities[&id(5)]);
    assert_eq!(
        outcome.project.authoring_locales,
        original.authoring_locales
    );
    assert_eq!(outcome.project.asset_store, original.asset_store);
    assert_eq!(
        ProjectRevision3::from_json(&outcome.canonical_project_json).unwrap(),
        outcome.project
    );
}

#[test]
fn exact_resolved_and_ambiguous_target_evidence_is_reported_before_slot_removal() {
    let resolved = basis(VoiceTargetResolution::Resolved {
        target: voice_target(
            "german_new.zip",
            "Npc/Asghan/GORE_ASGHAN_GREETING.ogg",
            0x41,
        ),
    });
    let resolved_outcome = applied(evaluate(&resolved, &request()));
    assert_eq!(
        resolved_outcome.removed_target_resolution,
        Revision3DialogVoiceSlotRemovalTargetResolutionV1::Resolved
    );

    let ambiguous = basis(VoiceTargetResolution::Ambiguous {
        candidates: vec![
            voice_target(
                "german_new.zip",
                "Npc/Asghan/GORE_ASGHAN_GREETING.ogg",
                0x42,
            ),
            voice_target("german_old.zip", "Legacy/GORE_ASGHAN_GREETING.ogg", 0x43),
        ],
    });
    let ambiguous_outcome = applied(evaluate(&ambiguous, &request()));
    assert_eq!(
        ambiguous_outcome.removed_target_resolution,
        Revision3DialogVoiceSlotRemovalTargetResolutionV1::Ambiguous
    );
}

#[test]
fn exact_head_project_target_line_slot_localization_and_locale_are_cas_bound() {
    let project = basis(VoiceTargetResolution::Unresolved);

    assert!(matches!(
        conflict(evaluate_with_head(&project, &request(), &head(0x32)).unwrap()),
        Revision3DialogVoiceSlotRemovalConflictV1::CurrentHeadMismatch
    ));

    let mut value = request();
    value.expected_project_id = project_id(0x11);
    assert!(matches!(
        conflict(evaluate(&project, &value)),
        Revision3DialogVoiceSlotRemovalConflictV1::ProjectIdentityMismatch { .. }
    ));
    let mut value = request();
    value.expected_revision += 1;
    assert!(matches!(
        conflict(evaluate(&project, &value)),
        Revision3DialogVoiceSlotRemovalConflictV1::ProjectRevisionConflict { .. }
    ));
    let mut value = request();
    value.expected_target = GameGenerationAnchor {
        executable: seal(0x21, 171_698_176),
    };
    assert!(matches!(
        conflict(evaluate(&project, &value)),
        Revision3DialogVoiceSlotRemovalConflictV1::ProjectTargetMismatch
    ));
    let mut value = request();
    value.expected_line_revision += 1;
    assert!(matches!(
        conflict(evaluate(&project, &value)),
        Revision3DialogVoiceSlotRemovalConflictV1::DialogLineRevisionConflict { .. }
    ));
    let mut value = request();
    value.localization_id = id(5);
    assert!(matches!(
        conflict(evaluate(&project, &value)),
        Revision3DialogVoiceSlotRemovalConflictV1::InvalidLocalizationReference { .. }
    ));
    let mut value = request();
    value.expected_loc_id = "GORE_OTHER".to_owned();
    assert!(matches!(
        conflict(evaluate(&project, &value)),
        Revision3DialogVoiceSlotRemovalConflictV1::LocalizationIdentityMismatch { .. }
    ));
    let mut value = request();
    value.locale = locale("en");
    assert!(matches!(
        conflict(evaluate(&project, &value)),
        Revision3DialogVoiceSlotRemovalConflictV1::VoiceSlotIdentityMismatch { .. }
    ));
    let mut value = request();
    value.slot_id = id(5);
    assert!(matches!(
        conflict(evaluate(&project, &value)),
        Revision3DialogVoiceSlotRemovalConflictV1::VoiceSlotIdentityMismatch { .. }
    ));
    let mut value = request();
    value.expected_slot_revision += 1;
    assert!(matches!(
        conflict(evaluate(&project, &value)),
        Revision3DialogVoiceSlotRemovalConflictV1::VoiceSlotRevisionConflict { .. }
    ));
}

#[test]
fn only_the_exact_managed_generated_slot_origin_can_be_removed() {
    let variants = [
        OriginRef::Imported {
            importer: "other-tool".to_owned(),
            source_seal: seal(0x51, 100),
            external_identity: None,
        },
        OriginRef::Generated {
            generator_id: "other-generator".to_owned(),
            generator_version: REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1,
            owner: TypedRef::new(project_id(0x10), id(3), EntityKind::DialogLine),
        },
        OriginRef::Generated {
            generator_id: REVISION3_VOICE_SLOT_GENERATOR_ID_V1.to_owned(),
            generator_version: REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1 + 1,
            owner: TypedRef::new(project_id(0x10), id(3), EntityKind::DialogLine),
        },
        OriginRef::Generated {
            generator_id: REVISION3_VOICE_SLOT_GENERATOR_ID_V1.to_owned(),
            generator_version: REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1,
            owner: TypedRef::new(project_id(0x10), id(5), EntityKind::DialogLine),
        },
    ];
    for origin in variants {
        let mut project = basis(VoiceTargetResolution::Unresolved);
        project.entities.get_mut(&id(4)).unwrap().origin = origin;
        assert!(matches!(
            conflict(evaluate(&project, &request())),
            Revision3DialogVoiceSlotRemovalConflictV1::VoiceSlotOriginMismatch { .. }
        ));
    }
}

fn add_take(project: &mut ProjectRevision3, selected: bool) {
    let take_id = id(6);
    let asset = AssetRef {
        sha256: digest(0x61),
        byte_len: 2048,
        logical_name: "asghan_take.ogg".to_owned(),
    };
    project.asset_store.assets.insert(
        asset.sha256,
        AssetMeta {
            byte_len: asset.byte_len,
            media_type: "audio/ogg".to_owned(),
        },
    );
    project.entities.insert(
        take_id,
        Entity {
            id: take_id,
            display_name: "Asghan take".to_owned(),
            origin: OriginRef::Imported {
                importer: "gore-authoring.ogg-import".to_owned(),
                source_seal: ContentSeal {
                    byte_len: asset.byte_len,
                    sha256: asset.sha256,
                },
                external_identity: None,
            },
            revision: 1,
            payload: EntityPayload::VoiceTake(VoiceTake {
                locale: locale("de"),
                asset,
                ogg: OggMetadata {
                    codec: OggCodec::Vorbis,
                    channels: 1,
                    sample_rate: 44_100,
                    pages: 2,
                    logical_streams: 1,
                },
                status: VoiceTakeStatus::Approved,
            }),
        },
    );
    let reference = TypedRef::new(project_id(0x10), take_id, EntityKind::VoiceTake);
    let slot = slot_mut(project);
    slot.candidates.push(reference.clone());
    if selected {
        slot.selected = Some(reference);
    }
}

#[test]
fn candidate_or_selected_take_blocks_slot_removal_without_mutating_input() {
    for selected in [false, true] {
        let mut project = basis(VoiceTargetResolution::Unresolved);
        add_take(&mut project, selected);
        let original = project.clone();
        let rejection = conflict(evaluate(&project, &request()));
        if selected {
            assert!(matches!(
                rejection,
                Revision3DialogVoiceSlotRemovalConflictV1::VoiceSlotHasSelection { .. }
            ));
        } else {
            assert!(matches!(
                rejection,
                Revision3DialogVoiceSlotRemovalConflictV1::VoiceSlotHasCandidates {
                    candidate_count: 1,
                    ..
                }
            ));
        }
        assert_eq!(project, original);
    }
}

#[test]
fn unexpected_local_backlink_blocks_removal_but_foreign_same_id_is_ignored() {
    let mut local = basis(VoiceTargetResolution::Unresolved);
    local.entities.get_mut(&id(5)).unwrap().origin = OriginRef::Generated {
        generator_id: "test-backlink".to_owned(),
        generator_version: 1,
        owner: TypedRef::new(project_id(0x10), id(4), EntityKind::VoiceSlot),
    };
    assert!(matches!(
        conflict(evaluate(&local, &request())),
        Revision3DialogVoiceSlotRemovalConflictV1::InvalidLocalBacklink {
            source_entity,
            role: Revision3ContentReferenceRoleV1::OriginOwner,
            ..
        } if source_entity == id(5)
    ));

    let mut foreign = basis(VoiceTargetResolution::Unresolved);
    foreign.entities.get_mut(&id(5)).unwrap().origin = OriginRef::Generated {
        generator_id: "foreign-backlink".to_owned(),
        generator_version: 1,
        owner: TypedRef::new(project_id(0x77), id(4), EntityKind::VoiceSlot),
    };
    let outcome = applied(evaluate(&foreign, &request()));
    assert!(!outcome.project.entities.contains_key(&id(4)));
    assert_eq!(
        outcome.project.entities[&id(5)].origin,
        foreign.entities[&id(5)].origin
    );
}

#[test]
fn invalid_identity_and_revision_overflow_reject_without_candidate() {
    let project = basis(VoiceTargetResolution::Unresolved);
    for mutate in [0u8, 1, 2] {
        let mut value = request();
        match mutate {
            0 => value.line_id = EntityId::from_bytes([0; 16]),
            1 => value.localization_id = value.line_id,
            2 => value.slot_id = value.localization_id,
            _ => unreachable!(),
        }
        assert!(matches!(
            conflict(evaluate(&project, &value)),
            Revision3DialogVoiceSlotRemovalConflictV1::InvalidEntityIdentity
        ));
    }

    let mut project_overflow = project.clone();
    project_overflow.revision = u64::MAX;
    let mut value = request();
    value.expected_revision = u64::MAX;
    assert!(matches!(
        conflict(evaluate(&project_overflow, &value)),
        Revision3DialogVoiceSlotRemovalConflictV1::ProjectRevisionOverflow
    ));

    let mut line_overflow = project;
    line_overflow.entities.get_mut(&id(3)).unwrap().revision = u64::MAX;
    let mut value = request();
    value.expected_line_revision = u64::MAX;
    assert!(matches!(
        conflict(evaluate(&line_overflow, &value)),
        Revision3DialogVoiceSlotRemovalConflictV1::DialogLineRevisionOverflow { .. }
    ));
}

#[test]
fn malformed_shared_slot_graph_fails_at_the_closed_project_boundary() {
    let mut project = basis(VoiceTargetResolution::Unresolved);
    let second_line_id = id(7);
    project.entities.insert(
        second_line_id,
        Entity {
            id: second_line_id,
            display_name: "Second line".to_owned(),
            origin: new_origin("DIA_SECOND"),
            revision: 0,
            payload: EntityPayload::DialogLine(DialogLine {
                localization: TypedRef::new(project_id(0x10), id(5), EntityKind::LocalizationEntry),
                speaker_hint: None,
                voice_slots: BTreeMap::from([(
                    locale("de"),
                    TypedRef::new(project_id(0x10), id(4), EntityKind::VoiceSlot),
                )]),
            }),
        },
    );
    let invalid_json = serde_json::to_string(&project).unwrap();
    let error = apply_revision3_dialog_voice_slot_removal_transaction_v1(
        &head(0x31),
        &invalid_json,
        &request().to_canonical_json().unwrap(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        Revision3DialogVoiceSlotRemovalErrorV1::InvalidProject(_)
    ));
}

#[test]
fn request_json_is_exact_canonical_duplicate_free_and_bounded() {
    let request = request();
    let canonical = request.to_canonical_json().unwrap();
    assert_eq!(
        Revision3DialogVoiceSlotRemovalRequestV1::from_json(&canonical).unwrap(),
        request
    );

    let duplicate = canonical.replacen(
        "\"expected_revision\":7",
        "\"expected_revision\":7,\"expected_revision\":7",
        1,
    );
    assert!(matches!(
        Revision3DialogVoiceSlotRemovalRequestV1::from_json(&duplicate),
        Err(Revision3DialogVoiceSlotRemovalRequestJsonErrorV1::InvalidJson(_))
    ));

    let noncanonical =
        canonical.replacen("\"expected_project_id\"", "\"z_expected_project_id\"", 1);
    assert!(matches!(
        Revision3DialogVoiceSlotRemovalRequestV1::from_json(&noncanonical),
        Err(Revision3DialogVoiceSlotRemovalRequestJsonErrorV1::InvalidJson(_))
    ));

    let padded = format!("{canonical} ");
    assert!(matches!(
        Revision3DialogVoiceSlotRemovalRequestV1::from_json(&padded),
        Err(Revision3DialogVoiceSlotRemovalRequestJsonErrorV1::NonCanonicalJson)
    ));

    let mut oversized = request;
    oversized.expected_loc_id =
        "x".repeat(MAX_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_JSON_BYTES_V1);
    assert!(matches!(
        oversized.to_canonical_json(),
        Err(Revision3DialogVoiceSlotRemovalRequestJsonErrorV1::InputTooLarge { .. })
    ));
}
