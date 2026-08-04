use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::model_revision3::{
    DialogLine, Entity, EntityKind, EntityPayload, LocalizationEntry, OggCodec, OggMetadata,
    OriginRef, SchemaRevisionV3, TypedRef, VoiceMemberProof, VoiceOperation, VoiceSlot, VoiceTake,
    VoiceTakeStatus, VoiceTarget, VoiceTargetResolution,
};
use gore_authoring::{
    apply_revision3_voice_target_resolution_transaction_v1,
    validate_revision3_voice_loc_id_basename_stem_v1, ArchiveSeal, AssetMeta, AssetRef,
    AssetStoreIndex, ContentSeal, EntityId, FormatV2, GameGenerationAnchor, LocaleCode, ProjectId,
    ProjectMeta, ProjectRevision3, ProjectRevision3ValidationError,
    Revision3VoiceLocIdBasenameStemErrorV1, Revision3VoiceTargetResolutionConflictV1,
    Revision3VoiceTargetResolutionEvaluationV1, Revision3VoiceTargetResolutionRequestJsonErrorV1,
    Revision3VoiceTargetResolutionRequestV1, Revision3VoiceTargetResolutionStateV1, Sha256Digest,
    WorkingHead, WorkingStoreFormat, MAX_REVISION3_VOICE_TARGET_ARCHIVE_CONTENT_BYTES_V1,
    MAX_REVISION3_VOICE_TARGET_LOC_ID_BYTES_V1, MAX_REVISION3_VOICE_TARGET_MATCHES_V1,
    MAX_REVISION3_VOICE_TARGET_MEMBER_UNCOMPRESSED_BYTES_V1,
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

fn seal(tag: u8, byte_len: u64) -> ContentSeal {
    ContentSeal {
        byte_len,
        sha256: digest(tag),
    }
}

fn target_generation() -> GameGenerationAnchor {
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

fn locale() -> LocaleCode {
    "de".parse().unwrap()
}

fn origin(tag: u8) -> OriginRef {
    OriginRef::Imported {
        importer: "voice-target-tests".to_owned(),
        source_seal: seal(tag, 100),
        external_identity: None,
    }
}

fn voice_target(archive: &str, member: &str, tag: u8) -> VoiceTarget {
    VoiceTarget {
        archive: archive.to_owned(),
        member: member.to_owned(),
        operation: VoiceOperation::Replace,
        archive_seal: ArchiveSeal {
            byte_len: 1024,
            sha256: digest(tag),
        },
        member_proof: VoiceMemberProof::Present {
            uncompressed_size: 8192,
            crc32: u32::from(tag),
        },
    }
}

fn basis() -> ProjectRevision3 {
    let localization_id = id(2);
    let line_id = id(3);
    let slot_id = id(4);
    let locale = locale();
    ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id: project_id(),
        revision: 7,
        meta: ProjectMeta {
            name: "Voice target transaction".to_owned(),
            version: "1.0.0".to_owned(),
            author: "tests".to_owned(),
        },
        target: target_generation(),
        authoring_locales: BTreeSet::from([locale.clone()]),
        entities: BTreeMap::from([
            (
                localization_id,
                Entity {
                    id: localization_id,
                    display_name: "Asghan line text".to_owned(),
                    origin: origin(2),
                    revision: 2,
                    payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                        loc_id: "GRD_263_ASGHAN_OPEN_INFO_06_02".to_owned(),
                        texts: BTreeMap::from([(locale.clone(), "Geh weiter.".to_owned())]),
                    }),
                },
            ),
            (
                line_id,
                Entity {
                    id: line_id,
                    display_name: "Asghan greeting".to_owned(),
                    origin: origin(3),
                    revision: 3,
                    payload: EntityPayload::DialogLine(DialogLine {
                        localization: TypedRef::new(
                            project_id(),
                            localization_id,
                            EntityKind::LocalizationEntry,
                        ),
                        speaker_hint: Some("Asghan".to_owned()),
                        voice_slots: BTreeMap::from([(
                            locale.clone(),
                            TypedRef::new(project_id(), slot_id, EntityKind::VoiceSlot),
                        )]),
                    }),
                },
            ),
            (
                slot_id,
                Entity {
                    id: slot_id,
                    display_name: "Asghan DE".to_owned(),
                    origin: origin(4),
                    revision: 4,
                    payload: EntityPayload::VoiceSlot(VoiceSlot {
                        locale,
                        target_resolution: VoiceTargetResolution::Unresolved,
                        candidates: Vec::new(),
                        selected: None,
                    }),
                },
            ),
        ]),
        asset_store: AssetStoreIndex::default(),
    }
}

fn request(matches: Vec<VoiceTarget>) -> Revision3VoiceTargetResolutionRequestV1 {
    Revision3VoiceTargetResolutionRequestV1 {
        expected_head: head(0x31),
        expected_project_id: project_id(),
        expected_revision: 7,
        expected_target: target_generation(),
        line_id: id(3),
        slot_id: id(4),
        locale: locale(),
        expected_loc_id: "GRD_263_ASGHAN_OPEN_INFO_06_02".to_owned(),
        matches,
    }
}

fn apply(
    project: &ProjectRevision3,
    request: &Revision3VoiceTargetResolutionRequestV1,
) -> Revision3VoiceTargetResolutionEvaluationV1 {
    apply_revision3_voice_target_resolution_transaction_v1(
        &head(0x31),
        &project.to_canonical_json().unwrap(),
        &request.to_canonical_json().unwrap(),
    )
    .unwrap()
}

#[test]
fn unique_native_match_is_bound_atomically_and_reopens_canonically() {
    let project = basis();
    let target = voice_target(
        "german_new.zip",
        "Npc/Asghan/GRD_263_ASGHAN_OPEN_INFO_06_02.ogg",
        0x41,
    );
    let request = request(vec![target.clone()]);
    let Revision3VoiceTargetResolutionEvaluationV1::Applied(outcome) = apply(&project, &request)
    else {
        panic!("expected applied target resolution")
    };

    assert_eq!(outcome.basis_head, head(0x31));
    assert_eq!(outcome.line_id, id(3));
    assert_eq!(outcome.localization_id, id(2));
    assert_eq!(outcome.slot_id, id(4));
    assert_eq!(outcome.locale, locale());
    assert_eq!(outcome.loc_id, request.expected_loc_id);
    assert_eq!(
        outcome.resolution_state,
        Revision3VoiceTargetResolutionStateV1::Resolved
    );
    assert_eq!(outcome.match_count, 1);
    assert_eq!(outcome.resolved_target, Some(target.clone()));
    assert_eq!(
        outcome.resolution,
        VoiceTargetResolution::Resolved {
            target: target.clone()
        }
    );
    assert_eq!(outcome.project.revision, 8);
    assert_eq!(outcome.project.entities[&id(4)].revision, 5);
    assert_eq!(outcome.project.entities[&id(3)].revision, 3);
    assert_eq!(outcome.project.entities[&id(2)].revision, 2);
    assert_eq!(
        ProjectRevision3::from_json(&outcome.canonical_project_json).unwrap(),
        outcome.project
    );
}

#[test]
fn native_match_cardinality_derives_unresolved_and_ambiguous_states() {
    let first = voice_target("german.zip", "Npc/Asghan/line.ogg", 0x42);
    let second = voice_target("german_new.zip", "Npc/Asghan/line.ogg", 0x43);
    for (matches, expected_state, expected_resolution) in [
        (
            Vec::new(),
            Revision3VoiceTargetResolutionStateV1::Unresolved,
            VoiceTargetResolution::Unresolved,
        ),
        (
            vec![first.clone(), second.clone()],
            Revision3VoiceTargetResolutionStateV1::Ambiguous,
            VoiceTargetResolution::Ambiguous {
                candidates: vec![first.clone(), second.clone()],
            },
        ),
    ] {
        let request = request(matches);
        let Revision3VoiceTargetResolutionEvaluationV1::Applied(outcome) =
            apply(&basis(), &request)
        else {
            panic!("expected applied cardinality mapping")
        };
        assert_eq!(outcome.resolution_state, expected_state);
        assert_eq!(outcome.resolution, expected_resolution);
        assert_eq!(outcome.match_count as usize, request.matches.len());
        assert_eq!(outcome.resolved_target, None);
    }
}

#[test]
fn canonical_request_and_exact_basis_bindings_reject_drift() {
    let canonical = request(Vec::new()).to_canonical_json().unwrap();
    assert!(matches!(
        Revision3VoiceTargetResolutionRequestV1::from_json(&format!(" {canonical}")),
        Err(Revision3VoiceTargetResolutionRequestJsonErrorV1::NonCanonicalJson)
    ));
    let duplicate = canonical.replacen(
        "\"expected_revision\":7",
        "\"expected_revision\":7,\"expected_revision\":7",
        1,
    );
    assert!(matches!(
        Revision3VoiceTargetResolutionRequestV1::from_json(&duplicate),
        Err(Revision3VoiceTargetResolutionRequestJsonErrorV1::InvalidJson(_))
    ));

    let mut cases = Vec::new();
    let mut wrong_head = request(Vec::new());
    wrong_head.expected_head = head(0x99);
    cases.push((
        wrong_head,
        Revision3VoiceTargetResolutionConflictV1::CurrentHeadMismatch,
    ));
    let mut wrong_project = request(Vec::new());
    wrong_project.expected_project_id = ProjectId::from_bytes([0x99; 16]);
    cases.push((
        wrong_project,
        Revision3VoiceTargetResolutionConflictV1::ProjectIdentityMismatch {
            expected: ProjectId::from_bytes([0x99; 16]),
            actual: project_id(),
        },
    ));
    let mut wrong_revision = request(Vec::new());
    wrong_revision.expected_revision = 6;
    cases.push((
        wrong_revision,
        Revision3VoiceTargetResolutionConflictV1::ProjectRevisionConflict {
            expected: 6,
            actual: 7,
        },
    ));
    let mut wrong_target = request(Vec::new());
    wrong_target.expected_target.executable.sha256 = digest(0x99);
    cases.push((
        wrong_target,
        Revision3VoiceTargetResolutionConflictV1::ProjectTargetMismatch,
    ));
    let mut wrong_loc = request(Vec::new());
    wrong_loc.expected_loc_id = "OTHER_LOC_ID".to_owned();
    cases.push((
        wrong_loc,
        Revision3VoiceTargetResolutionConflictV1::LocalizationIdentityMismatch {
            expected: "OTHER_LOC_ID".to_owned(),
            actual: "GRD_263_ASGHAN_OPEN_INFO_06_02".to_owned(),
        },
    ));
    let mut wrong_slot = request(Vec::new());
    wrong_slot.slot_id = id(9);
    cases.push((
        wrong_slot,
        Revision3VoiceTargetResolutionConflictV1::VoiceSlotIdentityMismatch { slot: id(9) },
    ));

    for (request, expected) in cases {
        let Revision3VoiceTargetResolutionEvaluationV1::Rejected(rejection) =
            apply(&basis(), &request)
        else {
            panic!("expected exact-binding rejection")
        };
        assert_eq!(rejection.conflict, expected);
    }
}

#[test]
fn exported_loc_id_basename_validator_is_ascii_portable_and_suffix_bounded() {
    let boundary = "L".repeat(MAX_REVISION3_VOICE_TARGET_LOC_ID_BYTES_V1);
    assert!(validate_revision3_voice_loc_id_basename_stem_v1(&boundary).is_ok());
    assert_eq!(
        validate_revision3_voice_loc_id_basename_stem_v1(""),
        Err(Revision3VoiceLocIdBasenameStemErrorV1::Empty)
    );
    assert_eq!(
        validate_revision3_voice_loc_id_basename_stem_v1(" LINE"),
        Err(Revision3VoiceLocIdBasenameStemErrorV1::NonCanonicalWhitespace)
    );
    assert_eq!(
        validate_revision3_voice_loc_id_basename_stem_v1("LÍNE"),
        Err(Revision3VoiceLocIdBasenameStemErrorV1::NonAscii)
    );
    assert_eq!(
        validate_revision3_voice_loc_id_basename_stem_v1(
            &"L".repeat(MAX_REVISION3_VOICE_TARGET_LOC_ID_BYTES_V1 + 1)
        ),
        Err(Revision3VoiceLocIdBasenameStemErrorV1::TooLong {
            actual: MAX_REVISION3_VOICE_TARGET_LOC_ID_BYTES_V1 + 1,
            max: MAX_REVISION3_VOICE_TARGET_LOC_ID_BYTES_V1,
        })
    );
    for unsafe_stem in ["folder/LINE", "folder\\LINE", "LINE:stream", "LINE.", "CON"] {
        assert_eq!(
            validate_revision3_voice_loc_id_basename_stem_v1(unsafe_stem),
            Err(Revision3VoiceLocIdBasenameStemErrorV1::UnsafeArchiveMember),
            "unsafe stem {unsafe_stem:?}"
        );
    }
}

#[test]
fn loc_id_stem_limit_reserves_exact_ogg_suffix_space() {
    let boundary_loc_id = "L".repeat(MAX_REVISION3_VOICE_TARGET_LOC_ID_BYTES_V1);
    let mut boundary_project = basis();
    let EntityPayload::LocalizationEntry(localization) =
        &mut boundary_project.entities.get_mut(&id(2)).unwrap().payload
    else {
        unreachable!()
    };
    localization.loc_id = boundary_loc_id.clone();
    let mut boundary_request = request(Vec::new());
    boundary_request.expected_loc_id = boundary_loc_id;
    assert!(matches!(
        apply(&boundary_project, &boundary_request),
        Revision3VoiceTargetResolutionEvaluationV1::Applied(_)
    ));

    let mut oversized_request = request(Vec::new());
    oversized_request.expected_loc_id = "L".repeat(MAX_REVISION3_VOICE_TARGET_LOC_ID_BYTES_V1 + 1);
    assert!(matches!(
        apply(&basis(), &oversized_request),
        Revision3VoiceTargetResolutionEvaluationV1::Rejected(rejection)
            if rejection.conflict
                == Revision3VoiceTargetResolutionConflictV1::InvalidExpectedLocId
    ));

    let mut oversized_project = basis();
    let EntityPayload::LocalizationEntry(localization) =
        &mut oversized_project.entities.get_mut(&id(2)).unwrap().payload
    else {
        unreachable!()
    };
    localization.loc_id = "L".repeat(MAX_REVISION3_VOICE_TARGET_LOC_ID_BYTES_V1 + 1);
    assert!(matches!(
        oversized_project.validate_closed_model(),
        Err(ProjectRevision3ValidationError::InvalidVoiceGraph { .. })
    ));

    let mut unsafe_request = request(Vec::new());
    unsafe_request.expected_loc_id = "folder/LINE".to_owned();
    assert!(matches!(
        apply(&basis(), &unsafe_request),
        Revision3VoiceTargetResolutionEvaluationV1::Rejected(rejection)
            if rejection.conflict
                == Revision3VoiceTargetResolutionConflictV1::InvalidExpectedLocId
    ));

    let mut non_ascii_project = basis();
    let EntityPayload::LocalizationEntry(localization) =
        &mut non_ascii_project.entities.get_mut(&id(2)).unwrap().payload
    else {
        unreachable!()
    };
    localization.loc_id = "LÍNE".to_owned();
    assert!(matches!(
        non_ascii_project.validate_closed_model(),
        Err(ProjectRevision3ValidationError::InvalidVoiceGraph { .. })
    ));
}

#[test]
fn native_evidence_rejects_add_absence_zeroes_unsafe_paths_duplicates_and_overflow() {
    let valid = voice_target("german.zip", "Npc/Asghan/line.ogg", 0x51);
    let mut invalid = Vec::new();

    let mut add = valid.clone();
    add.operation = VoiceOperation::Add;
    add.member_proof = VoiceMemberProof::Absent;
    invalid.push(add);
    let mut absent = valid.clone();
    absent.member_proof = VoiceMemberProof::Absent;
    invalid.push(absent);
    let mut zero_member = valid.clone();
    zero_member.member_proof = VoiceMemberProof::Present {
        uncompressed_size: 0,
        crc32: 0,
    };
    invalid.push(zero_member);
    let mut zero_seal = valid.clone();
    zero_seal.archive_seal.byte_len = 0;
    invalid.push(zero_seal);
    let mut zero_digest = valid.clone();
    zero_digest.archive_seal.sha256 = Sha256Digest::from_bytes([0; 32]);
    invalid.push(zero_digest);
    let mut oversized_archive = valid.clone();
    oversized_archive.archive_seal.byte_len =
        MAX_REVISION3_VOICE_TARGET_ARCHIVE_CONTENT_BYTES_V1 + 1;
    invalid.push(oversized_archive);
    let mut oversized_member = valid.clone();
    oversized_member.member_proof = VoiceMemberProof::Present {
        uncompressed_size: MAX_REVISION3_VOICE_TARGET_MEMBER_UNCOMPRESSED_BYTES_V1 + 1,
        crc32: 0x1020_3040,
    };
    invalid.push(oversized_member);
    for archive in ["german.pak", "../german.zip", "dir/german.zip", "CON.zip"] {
        let mut target = valid.clone();
        target.archive = archive.to_owned();
        invalid.push(target);
    }
    for member in [
        "line.wav",
        "../line.ogg",
        r"Npc\line.ogg",
        "/absolute.ogg",
        "Npc/CON.ogg",
    ] {
        let mut target = valid.clone();
        target.member = member.to_owned();
        invalid.push(target);
    }

    for target in invalid {
        let Revision3VoiceTargetResolutionEvaluationV1::Rejected(rejection) =
            apply(&basis(), &request(vec![target]))
        else {
            panic!("expected invalid native evidence rejection")
        };
        assert!(matches!(
            rejection.conflict,
            Revision3VoiceTargetResolutionConflictV1::InvalidNativeEvidence { .. }
        ));
    }

    let mut boundary = valid.clone();
    boundary.archive_seal.byte_len = MAX_REVISION3_VOICE_TARGET_ARCHIVE_CONTENT_BYTES_V1;
    boundary.member_proof = VoiceMemberProof::Present {
        uncompressed_size: MAX_REVISION3_VOICE_TARGET_MEMBER_UNCOMPRESSED_BYTES_V1,
        crc32: 0x5060_7080,
    };
    assert!(matches!(
        apply(&basis(), &request(vec![boundary])),
        Revision3VoiceTargetResolutionEvaluationV1::Applied(_)
    ));

    let duplicate = vec![valid.clone(), valid];
    assert!(matches!(
        apply(&basis(), &request(duplicate)),
        Revision3VoiceTargetResolutionEvaluationV1::Rejected(rejection)
            if matches!(rejection.conflict, Revision3VoiceTargetResolutionConflictV1::InvalidNativeEvidence { .. })
    ));
    let too_many = (0..=MAX_REVISION3_VOICE_TARGET_MATCHES_V1)
        .map(|index| {
            voice_target(
                &format!("voice_{index}.zip"),
                &format!("Npc/line_{index}.ogg"),
                ((index % 250) + 1) as u8,
            )
        })
        .collect();
    assert!(matches!(
        apply(&basis(), &request(too_many)),
        Revision3VoiceTargetResolutionEvaluationV1::Rejected(rejection)
            if matches!(rejection.conflict, Revision3VoiceTargetResolutionConflictV1::InvalidNativeEvidence { .. })
    ));
}

#[test]
fn closed_model_enforces_voice_graph_selection_assets_and_target_uniqueness() {
    let mut missing_locale = basis();
    missing_locale.authoring_locales.clear();
    assert!(matches!(
        missing_locale.validate_closed_model(),
        Err(ProjectRevision3ValidationError::InvalidVoiceGraph { .. })
    ));

    let mut orphan_slot = basis();
    let EntityPayload::DialogLine(line) =
        &mut orphan_slot.entities.get_mut(&id(3)).unwrap().payload
    else {
        unreachable!()
    };
    line.voice_slots.clear();
    assert!(matches!(
        orphan_slot.validate_closed_model(),
        Err(ProjectRevision3ValidationError::InvalidVoiceGraph { .. })
    ));

    let mut selected_unapproved = basis();
    let asset = digest(0x61);
    selected_unapproved.asset_store.assets.insert(
        asset,
        AssetMeta {
            byte_len: 4096,
            media_type: "audio/ogg".to_owned(),
        },
    );
    selected_unapproved.entities.insert(
        id(5),
        Entity {
            id: id(5),
            display_name: "Take".to_owned(),
            origin: origin(5),
            revision: 0,
            payload: EntityPayload::VoiceTake(VoiceTake {
                locale: locale(),
                asset: AssetRef {
                    sha256: asset,
                    byte_len: 4096,
                    logical_name: "take.ogg".to_owned(),
                },
                ogg: OggMetadata {
                    codec: OggCodec::Vorbis,
                    channels: 1,
                    sample_rate: 48_000,
                    pages: 2,
                    logical_streams: 1,
                },
                status: VoiceTakeStatus::Reviewed,
            }),
        },
    );
    let take_ref = TypedRef::new(project_id(), id(5), EntityKind::VoiceTake);
    let EntityPayload::VoiceSlot(slot) = &mut selected_unapproved
        .entities
        .get_mut(&id(4))
        .unwrap()
        .payload
    else {
        unreachable!()
    };
    slot.candidates.push(take_ref.clone());
    slot.selected = Some(take_ref);
    selected_unapproved.validate_closed_model().unwrap();
    let EntityPayload::VoiceTake(take) = &mut selected_unapproved
        .entities
        .get_mut(&id(5))
        .unwrap()
        .payload
    else {
        unreachable!()
    };
    take.status = VoiceTakeStatus::Approved;
    selected_unapproved
        .asset_store
        .assets
        .get_mut(&asset)
        .unwrap()
        .media_type = "application/octet-stream".into();
    assert!(matches!(
        selected_unapproved.validate_closed_model(),
        Err(ProjectRevision3ValidationError::InvalidVoiceTake { .. })
    ));

    let mut invalid_target = basis();
    let EntityPayload::VoiceSlot(slot) =
        &mut invalid_target.entities.get_mut(&id(4)).unwrap().payload
    else {
        unreachable!()
    };
    slot.target_resolution = VoiceTargetResolution::Ambiguous {
        candidates: vec![voice_target("german.zip", "line.ogg", 0x62)],
    };
    assert!(matches!(
        invalid_target.validate_closed_model(),
        Err(ProjectRevision3ValidationError::InvalidVoiceTarget { .. })
    ));

    let duplicate = voice_target("german.zip", "Npc/line.ogg", 0x63);
    let mut duplicate_targets = basis();
    let EntityPayload::VoiceSlot(first) =
        &mut duplicate_targets.entities.get_mut(&id(4)).unwrap().payload
    else {
        unreachable!()
    };
    first.target_resolution = VoiceTargetResolution::Resolved {
        target: duplicate.clone(),
    };
    duplicate_targets.entities.insert(
        id(6),
        Entity {
            id: id(6),
            display_name: "Second line".to_owned(),
            origin: origin(6),
            revision: 0,
            payload: EntityPayload::DialogLine(DialogLine {
                localization: TypedRef::new(project_id(), id(2), EntityKind::LocalizationEntry),
                speaker_hint: None,
                voice_slots: BTreeMap::from([(
                    locale(),
                    TypedRef::new(project_id(), id(7), EntityKind::VoiceSlot),
                )]),
            }),
        },
    );
    let mut folded_duplicate = duplicate;
    folded_duplicate.archive = "GERMAN.ZIP".to_owned();
    folded_duplicate.member = "npc/LINE.OGG".to_owned();
    duplicate_targets.entities.insert(
        id(7),
        Entity {
            id: id(7),
            display_name: "Second slot".to_owned(),
            origin: origin(7),
            revision: 0,
            payload: EntityPayload::VoiceSlot(VoiceSlot {
                locale: locale(),
                target_resolution: VoiceTargetResolution::Resolved {
                    target: folded_duplicate,
                },
                candidates: Vec::new(),
                selected: None,
            }),
        },
    );
    assert!(matches!(
        duplicate_targets.validate_closed_model(),
        Err(ProjectRevision3ValidationError::DuplicateVoiceTarget { .. })
    ));
}

#[test]
fn overflow_and_existing_duplicate_target_are_rejected_before_mutation() {
    let mut project_overflow = basis();
    project_overflow.revision = u64::MAX;
    let mut overflow_request = request(Vec::new());
    overflow_request.expected_revision = u64::MAX;
    assert!(matches!(
        apply(&project_overflow, &overflow_request),
        Revision3VoiceTargetResolutionEvaluationV1::Rejected(rejection)
            if rejection.conflict == Revision3VoiceTargetResolutionConflictV1::ProjectRevisionOverflow
    ));

    let mut slot_overflow = basis();
    slot_overflow.entities.get_mut(&id(4)).unwrap().revision = u64::MAX;
    assert!(matches!(
        apply(&slot_overflow, &request(Vec::new())),
        Revision3VoiceTargetResolutionEvaluationV1::Rejected(rejection)
            if rejection.conflict == Revision3VoiceTargetResolutionConflictV1::VoiceSlotRevisionOverflow { slot: id(4) }
    ));

    let existing = voice_target("german.zip", "Npc/line.ogg", 0x71);
    let mut duplicate_project = basis();
    duplicate_project.entities.insert(
        id(6),
        Entity {
            id: id(6),
            display_name: "Second line".to_owned(),
            origin: origin(6),
            revision: 0,
            payload: EntityPayload::DialogLine(DialogLine {
                localization: TypedRef::new(project_id(), id(2), EntityKind::LocalizationEntry),
                speaker_hint: None,
                voice_slots: BTreeMap::from([(
                    locale(),
                    TypedRef::new(project_id(), id(7), EntityKind::VoiceSlot),
                )]),
            }),
        },
    );
    duplicate_project.entities.insert(
        id(7),
        Entity {
            id: id(7),
            display_name: "Existing target".to_owned(),
            origin: origin(7),
            revision: 0,
            payload: EntityPayload::VoiceSlot(VoiceSlot {
                locale: locale(),
                target_resolution: VoiceTargetResolution::Resolved {
                    target: existing.clone(),
                },
                candidates: Vec::new(),
                selected: None,
            }),
        },
    );
    let request = request(vec![existing]);
    assert!(matches!(
        apply(&duplicate_project, &request),
        Revision3VoiceTargetResolutionEvaluationV1::Rejected(rejection)
            if rejection.conflict == Revision3VoiceTargetResolutionConflictV1::DuplicateResolvedTarget { existing_slot: id(7) }
    ));
}
