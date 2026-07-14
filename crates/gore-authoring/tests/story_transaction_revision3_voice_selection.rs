use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::model_revision3::{
    DialogLine, Entity, EntityKind, EntityPayload, LocalizationEntry, OggCodec, OggMetadata,
    OriginRef, SchemaRevisionV3, TypedRef, VoiceMemberProof, VoiceOperation, VoiceSlot, VoiceTake,
    VoiceTakeStatus, VoiceTarget, VoiceTargetResolution,
};
use gore_authoring::{
    apply_revision3_voice_take_selection_transaction_v1, plan_revision3_voice_build_v1,
    ArchiveSeal, AssetMeta, AssetRef, AssetStoreIndex, ContentSeal, EntityId, FormatV2,
    GameGenerationAnchor, LocaleCode, ProjectId, ProjectMeta, ProjectRevision3,
    ProjectRevision3JsonError, Revision3VoiceBuildBlockReasonV1,
    Revision3VoiceBuildPlanEvaluationV1, Revision3VoiceTakeSelectionBuildStatusV1,
    Revision3VoiceTakeSelectionConflictV1, Revision3VoiceTakeSelectionErrorV1,
    Revision3VoiceTakeSelectionEvaluationV1, Revision3VoiceTakeSelectionRequestJsonErrorV1,
    Revision3VoiceTakeSelectionRequestV1, Revision3VoiceTakeSelectionRuntimeStatusV1, Sha256Digest,
    WorkingHead, WorkingStoreFormat, MAX_PROJECT_JSON_BYTES,
    MAX_REVISION3_VOICE_TAKE_SELECTION_REQUEST_JSON_BYTES_V1,
};

const LOC_ID_ONE: &str = "GRD_263_ASGHAN_OPEN_INFO_06_02";
const LOC_ID_TWO: &str = "STT_302_VIPER_GREET_INFO_11_02";

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

fn head(tag: u8) -> WorkingHead {
    WorkingHead {
        store_format: WorkingStoreFormat,
        snapshot: seal(tag, 4096),
    }
}

fn basis_head() -> WorkingHead {
    head(0x31)
}

fn target_generation() -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: seal(0x20, 171_698_176),
    }
}

fn locale() -> LocaleCode {
    "de".parse().unwrap()
}

fn origin(tag: u8) -> OriginRef {
    OriginRef::Imported {
        importer: "voice-selection-tests".to_owned(),
        source_seal: seal(tag, 100),
        external_identity: None,
    }
}

fn target(tag: u8, member: &str) -> VoiceTargetResolution {
    VoiceTargetResolution::Resolved {
        target: VoiceTarget {
            archive: "german_new.zip".to_owned(),
            member: member.to_owned(),
            operation: VoiceOperation::Replace,
            archive_seal: ArchiveSeal {
                byte_len: 2048,
                sha256: digest(0x50),
            },
            member_proof: VoiceMemberProof::Present {
                uncompressed_size: 8192,
                crc32: u32::from(tag),
            },
        },
    }
}

fn asset(tag: u8, logical_name: &str) -> AssetRef {
    AssetRef {
        sha256: digest(tag),
        byte_len: u64::from(tag) * 100,
        logical_name: logical_name.to_owned(),
    }
}

fn take_entity(
    take_id: EntityId,
    tag: u8,
    name: &str,
    logical_name: &str,
    status: VoiceTakeStatus,
) -> Entity {
    Entity {
        id: take_id,
        display_name: name.to_owned(),
        origin: origin(tag),
        revision: u64::from(tag),
        payload: EntityPayload::VoiceTake(VoiceTake {
            locale: locale(),
            asset: asset(tag, logical_name),
            ogg: OggMetadata {
                codec: OggCodec::Vorbis,
                channels: 1,
                sample_rate: 48_000,
                pages: 3,
                logical_streams: 1,
            },
            status,
        }),
    }
}

/// Two complete lines. Take B is intentionally a candidate of, and selected by, both slots.
fn basis() -> ProjectRevision3 {
    let project_id = project_id();
    let locale = locale();
    let take_a = asset(0x41, "asghan-a.ogg");
    let take_b = asset(0x42, "shared-b.ogg");
    let take_c = asset(0x43, "viper-c.ogg");
    ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id,
        revision: 11,
        meta: ProjectMeta {
            name: "VoiceSelection".to_owned(),
            version: "1.0.0".to_owned(),
            author: "tests".to_owned(),
        },
        target: target_generation(),
        authoring_locales: BTreeSet::from([locale.clone()]),
        entities: BTreeMap::from([
            (
                id(1),
                Entity {
                    id: id(1),
                    display_name: "Asghan line text".to_owned(),
                    origin: origin(1),
                    revision: 1,
                    payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                        loc_id: LOC_ID_ONE.to_owned(),
                        texts: BTreeMap::from([(locale.clone(), "Geh weiter.".to_owned())]),
                    }),
                },
            ),
            (
                id(2),
                Entity {
                    id: id(2),
                    display_name: "Asghan greeting".to_owned(),
                    origin: origin(2),
                    revision: 2,
                    payload: EntityPayload::DialogLine(DialogLine {
                        localization: TypedRef::new(
                            project_id,
                            id(1),
                            EntityKind::LocalizationEntry,
                        ),
                        speaker_hint: Some("Asghan".to_owned()),
                        voice_slots: BTreeMap::from([(
                            locale.clone(),
                            TypedRef::new(project_id, id(3), EntityKind::VoiceSlot),
                        )]),
                    }),
                },
            ),
            (
                id(3),
                Entity {
                    id: id(3),
                    display_name: "Asghan DE".to_owned(),
                    origin: origin(3),
                    revision: 4,
                    payload: EntityPayload::VoiceSlot(VoiceSlot {
                        locale: locale.clone(),
                        target_resolution: target(
                            0x31,
                            "Npc/Asghan/GRD_263_ASGHAN_OPEN_INFO_06_02.ogg",
                        ),
                        candidates: vec![
                            TypedRef::new(project_id, id(4), EntityKind::VoiceTake),
                            TypedRef::new(project_id, id(5), EntityKind::VoiceTake),
                        ],
                        selected: Some(TypedRef::new(project_id, id(4), EntityKind::VoiceTake)),
                    }),
                },
            ),
            (
                id(4),
                take_entity(
                    id(4),
                    0x41,
                    "Asghan take A",
                    "asghan-a.ogg",
                    VoiceTakeStatus::Approved,
                ),
            ),
            (
                id(5),
                take_entity(
                    id(5),
                    0x42,
                    "Shared take B",
                    "shared-b.ogg",
                    VoiceTakeStatus::Approved,
                ),
            ),
            (
                id(6),
                Entity {
                    id: id(6),
                    display_name: "Viper line text".to_owned(),
                    origin: origin(6),
                    revision: 6,
                    payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                        loc_id: LOC_ID_TWO.to_owned(),
                        texts: BTreeMap::from([(locale.clone(), "Na gut.".to_owned())]),
                    }),
                },
            ),
            (
                id(7),
                Entity {
                    id: id(7),
                    display_name: "Viper greeting".to_owned(),
                    origin: origin(7),
                    revision: 7,
                    payload: EntityPayload::DialogLine(DialogLine {
                        localization: TypedRef::new(
                            project_id,
                            id(6),
                            EntityKind::LocalizationEntry,
                        ),
                        speaker_hint: Some("Viper".to_owned()),
                        voice_slots: BTreeMap::from([(
                            locale.clone(),
                            TypedRef::new(project_id, id(8), EntityKind::VoiceSlot),
                        )]),
                    }),
                },
            ),
            (
                id(8),
                Entity {
                    id: id(8),
                    display_name: "Viper DE".to_owned(),
                    origin: origin(8),
                    revision: 8,
                    payload: EntityPayload::VoiceSlot(VoiceSlot {
                        locale: locale.clone(),
                        target_resolution: target(
                            0x32,
                            "Npc/Viper/STT_302_VIPER_GREET_INFO_11_02.ogg",
                        ),
                        candidates: vec![
                            TypedRef::new(project_id, id(5), EntityKind::VoiceTake),
                            TypedRef::new(project_id, id(9), EntityKind::VoiceTake),
                        ],
                        selected: Some(TypedRef::new(project_id, id(5), EntityKind::VoiceTake)),
                    }),
                },
            ),
            (
                id(9),
                take_entity(
                    id(9),
                    0x43,
                    "Viper take C",
                    "viper-c.ogg",
                    VoiceTakeStatus::Approved,
                ),
            ),
        ]),
        asset_store: AssetStoreIndex {
            assets: BTreeMap::from([
                (
                    take_a.sha256,
                    AssetMeta {
                        byte_len: take_a.byte_len,
                        media_type: "audio/ogg".to_owned(),
                    },
                ),
                (
                    take_b.sha256,
                    AssetMeta {
                        byte_len: take_b.byte_len,
                        media_type: "audio/ogg".to_owned(),
                    },
                ),
                (
                    take_c.sha256,
                    AssetMeta {
                        byte_len: take_c.byte_len,
                        media_type: "audio/ogg".to_owned(),
                    },
                ),
            ]),
        },
    }
}

fn request(
    project: &ProjectRevision3,
    expected_selected_take_id: Option<EntityId>,
    selected_take_id: Option<EntityId>,
) -> Revision3VoiceTakeSelectionRequestV1 {
    Revision3VoiceTakeSelectionRequestV1 {
        expected_head: basis_head(),
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        expected_target: project.target.clone(),
        line_id: id(2),
        slot_id: id(3),
        expected_slot_revision: project.entities[&id(3)].revision,
        locale: locale(),
        expected_loc_id: LOC_ID_ONE.to_owned(),
        expected_selected_take_id,
        selected_take_id,
    }
}

fn apply(
    project: &ProjectRevision3,
    request: &Revision3VoiceTakeSelectionRequestV1,
) -> Revision3VoiceTakeSelectionEvaluationV1 {
    apply_with_head(project, request, &basis_head())
}

fn apply_with_head(
    project: &ProjectRevision3,
    request: &Revision3VoiceTakeSelectionRequestV1,
    exact_head: &WorkingHead,
) -> Revision3VoiceTakeSelectionEvaluationV1 {
    apply_revision3_voice_take_selection_transaction_v1(
        exact_head,
        &project.to_canonical_json().unwrap(),
        &request.to_canonical_json().unwrap(),
    )
    .unwrap()
}

fn conflict(
    project: &ProjectRevision3,
    request: &Revision3VoiceTakeSelectionRequestV1,
) -> Revision3VoiceTakeSelectionConflictV1 {
    let before = project.clone();
    let canonical_before = project.to_canonical_json().unwrap();
    let Revision3VoiceTakeSelectionEvaluationV1::Rejected(rejection) = apply(project, request)
    else {
        panic!("expected rejected Voice take selection")
    };
    assert_eq!(project, &before, "rejection mutated the caller's project");
    assert_eq!(
        project.to_canonical_json().unwrap(),
        canonical_before,
        "rejection changed the caller's canonical bytes"
    );
    rejection.conflict
}

fn slot(project: &ProjectRevision3, slot_id: EntityId) -> &VoiceSlot {
    let EntityPayload::VoiceSlot(slot) = &project.entities[&slot_id].payload else {
        panic!("expected VoiceSlot")
    };
    slot
}

fn slot_mut(project: &mut ProjectRevision3, slot_id: EntityId) -> &mut VoiceSlot {
    let EntityPayload::VoiceSlot(slot) = &mut project.entities.get_mut(&slot_id).unwrap().payload
    else {
        panic!("expected VoiceSlot")
    };
    slot
}

fn take_mut(project: &mut ProjectRevision3, take_id: EntityId) -> &mut VoiceTake {
    let EntityPayload::VoiceTake(take) = &mut project.entities.get_mut(&take_id).unwrap().payload
    else {
        panic!("expected VoiceTake")
    };
    take
}

#[test]
fn select_changes_only_project_revision_slot_revision_and_selected_reference() {
    let project = basis();
    let request = request(&project, Some(id(4)), Some(id(5)));
    let Revision3VoiceTakeSelectionEvaluationV1::Applied(outcome) = apply(&project, &request)
    else {
        panic!("expected applied Voice take selection")
    };

    let mut expected = project.clone();
    expected.revision += 1;
    expected.entities.get_mut(&id(3)).unwrap().revision += 1;
    slot_mut(&mut expected, id(3)).selected =
        Some(TypedRef::new(project_id(), id(5), EntityKind::VoiceTake));

    assert_eq!(outcome.project, expected);
    assert_eq!(project.revision, 11);
    assert_eq!(outcome.basis_head, basis_head());
    assert_eq!(outcome.line_id, id(2));
    assert_eq!(outcome.localization_id, id(1));
    assert_eq!(outcome.slot_id, id(3));
    assert_eq!(outcome.slot_revision, 5);
    assert_eq!(outcome.locale, locale());
    assert_eq!(outcome.loc_id, LOC_ID_ONE);
    assert_eq!(outcome.previous_selected_take_id, Some(id(4)));
    assert_eq!(outcome.selected_take_id, Some(id(5)));
    assert_eq!(
        outcome.build_status,
        Revision3VoiceTakeSelectionBuildStatusV1::Blocked
    );
    assert_eq!(
        outcome.runtime_status,
        Revision3VoiceTakeSelectionRuntimeStatusV1::RuntimeUnqualified
    );
    assert_eq!(
        ProjectRevision3::from_json(&outcome.canonical_project_json).unwrap(),
        outcome.project
    );

    // Take B is also selected by the Viper slot. Its reuse, entity, asset, and that entire slot
    // must remain byte/structure-identical while only Asghan's slot changes selection.
    assert_eq!(outcome.project.entities[&id(8)], project.entities[&id(8)]);
    assert_eq!(outcome.project.entities[&id(5)], project.entities[&id(5)]);
    assert_eq!(outcome.project.asset_store, project.asset_store);
    assert_eq!(
        slot(&outcome.project, id(3)).candidates,
        slot(&project, id(3)).candidates
    );
    assert_eq!(
        slot(&outcome.project, id(3)).target_resolution,
        slot(&project, id(3)).target_resolution
    );
}

#[test]
fn clear_changes_only_requested_slot_and_build_plan_reports_missing_selection() {
    let project = basis();
    let request = request(&project, Some(id(4)), None);
    let Revision3VoiceTakeSelectionEvaluationV1::Applied(outcome) = apply(&project, &request)
    else {
        panic!("expected applied Voice take clear")
    };
    assert_eq!(slot(&outcome.project, id(3)).selected, None);
    assert_eq!(
        slot(&outcome.project, id(8))
            .selected
            .as_ref()
            .map(|item| item.id),
        Some(id(5)),
        "clearing one slot must not clear a shared take from another slot"
    );
    assert_eq!(outcome.previous_selected_take_id, Some(id(4)));
    assert_eq!(outcome.selected_take_id, None);

    let Revision3VoiceBuildPlanEvaluationV1::Blocked { report } =
        plan_revision3_voice_build_v1(&outcome.project).unwrap()
    else {
        panic!("cleared selection must block the all-or-nothing Voice build")
    };
    assert!(report.blockers.iter().any(|blocker| {
        blocker.slot_id == Some(id(3))
            && blocker.reason == Revision3VoiceBuildBlockReasonV1::MissingSelectedTake
    }));
    assert!(!report.blockers.iter().any(|blocker| {
        blocker.slot_id == Some(id(8))
            && blocker.reason == Revision3VoiceBuildBlockReasonV1::MissingSelectedTake
    }));
}

#[test]
fn changed_selection_drives_the_exact_different_asset_into_the_offline_build_plan() {
    let project = basis();
    let request = request(&project, Some(id(4)), Some(id(5)));
    let Revision3VoiceTakeSelectionEvaluationV1::Applied(outcome) = apply(&project, &request)
    else {
        panic!("expected applied Voice take selection")
    };
    let Revision3VoiceBuildPlanEvaluationV1::Ready { plan } =
        plan_revision3_voice_build_v1(&outcome.project).unwrap()
    else {
        panic!("fully resolved approved fixture must produce a ready build plan")
    };
    let edit = plan
        .edits
        .iter()
        .find(|edit| edit.slot_id == id(3))
        .unwrap();
    let EntityPayload::VoiceTake(selected_take) = &outcome.project.entities[&id(5)].payload else {
        unreachable!()
    };
    assert_eq!(edit.take_id, id(5));
    assert_eq!(edit.asset, selected_take.asset);
    assert_ne!(edit.asset.sha256, digest(0x41));
}

#[test]
fn every_nonapproved_status_is_rejected_without_mutating_input() {
    for status in [
        VoiceTakeStatus::Draft,
        VoiceTakeStatus::Recorded,
        VoiceTakeStatus::Reviewed,
    ] {
        let mut project = basis();
        take_mut(&mut project, id(5)).status = status;
        let request = request(&project, Some(id(4)), Some(id(5)));
        assert_eq!(
            conflict(&project, &request),
            Revision3VoiceTakeSelectionConflictV1::SelectedTakeNotApproved { take: id(5) }
        );
    }
}

#[test]
fn noncandidate_and_zero_or_colliding_take_identities_fail_closed() {
    let project = basis();
    let mut request = request(&project, Some(id(4)), Some(id(9)));
    assert_eq!(
        conflict(&project, &request),
        Revision3VoiceTakeSelectionConflictV1::SelectedTakeNotCandidate { take: id(9) }
    );

    for take in [EntityId::from_bytes([0; 16]), id(2), id(3)] {
        request.selected_take_id = Some(take);
        assert_eq!(
            conflict(&project, &request),
            Revision3VoiceTakeSelectionConflictV1::InvalidTakeIdentity { take }
        );
    }
}

#[test]
fn exact_head_project_revision_target_slot_revision_and_current_selection_are_cas_bound() {
    let project = basis();
    let base = request(&project, Some(id(4)), Some(id(5)));

    let Revision3VoiceTakeSelectionEvaluationV1::Rejected(rejection) =
        apply_with_head(&project, &base, &head(0x32))
    else {
        panic!("expected stale head rejection")
    };
    assert_eq!(
        rejection.conflict,
        Revision3VoiceTakeSelectionConflictV1::CurrentHeadMismatch
    );

    let mut request = base.clone();
    request.expected_project_id = ProjectId::from_bytes([0x99; 16]);
    assert!(matches!(
        conflict(&project, &request),
        Revision3VoiceTakeSelectionConflictV1::ProjectIdentityMismatch { .. }
    ));

    request = base.clone();
    request.expected_revision += 1;
    assert_eq!(
        conflict(&project, &request),
        Revision3VoiceTakeSelectionConflictV1::ProjectRevisionConflict {
            expected: 12,
            actual: 11,
        }
    );

    request = base.clone();
    request.expected_target.executable.sha256 = digest(0x99);
    assert_eq!(
        conflict(&project, &request),
        Revision3VoiceTakeSelectionConflictV1::ProjectTargetMismatch
    );

    request = base.clone();
    request.expected_slot_revision += 1;
    assert_eq!(
        conflict(&project, &request),
        Revision3VoiceTakeSelectionConflictV1::VoiceSlotRevisionConflict {
            expected: 5,
            actual: 4,
        }
    );

    request = base;
    request.expected_selected_take_id = None;
    assert_eq!(
        conflict(&project, &request),
        Revision3VoiceTakeSelectionConflictV1::CurrentSelectionMismatch {
            expected: None,
            actual: Some(id(4)),
        }
    );
}

#[test]
fn line_localization_locale_slot_and_unique_owner_intent_are_bound() {
    let project = basis();
    let base = request(&project, Some(id(4)), Some(id(5)));

    let mut request = base.clone();
    request.expected_loc_id = LOC_ID_TWO.to_owned();
    assert_eq!(
        conflict(&project, &request),
        Revision3VoiceTakeSelectionConflictV1::LocalizationIdentityMismatch {
            expected: LOC_ID_TWO.to_owned(),
            actual: LOC_ID_ONE.to_owned(),
        }
    );

    request = base.clone();
    request.locale = "en".parse().unwrap();
    assert_eq!(
        conflict(&project, &request),
        Revision3VoiceTakeSelectionConflictV1::VoiceSlotIdentityMismatch { slot: id(3) }
    );

    request = base.clone();
    request.line_id = id(7);
    request.expected_loc_id = LOC_ID_TWO.to_owned();
    assert_eq!(
        conflict(&project, &request),
        Revision3VoiceTakeSelectionConflictV1::VoiceSlotIdentityMismatch { slot: id(3) }
    );

    request = base;
    request.slot_id = id(8);
    request.expected_slot_revision = project.entities[&id(8)].revision;
    assert_eq!(
        conflict(&project, &request),
        Revision3VoiceTakeSelectionConflictV1::VoiceSlotIdentityMismatch { slot: id(8) }
    );
}

#[test]
fn no_op_and_revision_overflows_are_rejected() {
    let project = basis();
    assert_eq!(
        conflict(&project, &request(&project, Some(id(4)), Some(id(4)))),
        Revision3VoiceTakeSelectionConflictV1::NoChanges
    );

    let mut clear = project.clone();
    slot_mut(&mut clear, id(3)).selected = None;
    assert_eq!(
        conflict(&clear, &request(&clear, None, None)),
        Revision3VoiceTakeSelectionConflictV1::NoChanges
    );

    let mut project_overflow = project.clone();
    project_overflow.revision = u64::MAX;
    assert_eq!(
        conflict(
            &project_overflow,
            &request(&project_overflow, Some(id(4)), Some(id(5)))
        ),
        Revision3VoiceTakeSelectionConflictV1::ProjectRevisionOverflow
    );

    let mut slot_overflow = project;
    slot_overflow.entities.get_mut(&id(3)).unwrap().revision = u64::MAX;
    assert_eq!(
        conflict(
            &slot_overflow,
            &request(&slot_overflow, Some(id(4)), Some(id(5)))
        ),
        Revision3VoiceTakeSelectionConflictV1::VoiceSlotRevisionOverflow { slot: id(3) }
    );
}

#[test]
fn request_is_bounded_duplicate_safe_canonical_and_field_ordered() {
    let project = basis();
    let request = request(&project, Some(id(4)), Some(id(5)));
    let canonical = request.to_canonical_json().unwrap();
    assert_eq!(
        Revision3VoiceTakeSelectionRequestV1::from_json(&canonical).unwrap(),
        request
    );

    let ordered_fields = [
        "\"expected_head\"",
        "\"expected_project_id\"",
        "\"expected_revision\"",
        "\"expected_target\"",
        "\"line_id\"",
        "\"slot_id\"",
        "\"expected_slot_revision\"",
        "\"locale\"",
        "\"expected_loc_id\"",
        "\"expected_selected_take_id\"",
        "\"selected_take_id\"",
    ];
    let mut previous = 0;
    for (index, field) in ordered_fields.into_iter().enumerate() {
        let position = canonical.find(field).unwrap();
        if index > 0 {
            assert!(
                position > previous,
                "request field order drifted at {field}"
            );
        }
        previous = position;
    }

    assert!(matches!(
        Revision3VoiceTakeSelectionRequestV1::from_json(&format!(" {canonical}")),
        Err(Revision3VoiceTakeSelectionRequestJsonErrorV1::NonCanonicalJson)
    ));
    let duplicate = canonical.replacen(
        "\"expected_revision\":11",
        "\"expected_revision\":11,\"expected_revision\":11",
        1,
    );
    assert!(matches!(
        Revision3VoiceTakeSelectionRequestV1::from_json(&duplicate),
        Err(Revision3VoiceTakeSelectionRequestJsonErrorV1::InvalidJson(
            _
        ))
    ));
    let unknown = canonical.replacen('{', "{\"unknown\":true,", 1);
    assert!(matches!(
        Revision3VoiceTakeSelectionRequestV1::from_json(&unknown),
        Err(Revision3VoiceTakeSelectionRequestJsonErrorV1::InvalidJson(
            _
        ))
    ));
    let selected_field = canonical.rfind(",\"selected_take_id\":").unwrap();
    let missing = format!("{}}}", &canonical[..selected_field]);
    assert!(matches!(
        Revision3VoiceTakeSelectionRequestV1::from_json(&missing),
        Err(Revision3VoiceTakeSelectionRequestJsonErrorV1::NonCanonicalJson)
    ));
    let wrong_type = canonical.replacen(
        "\"expected_slot_revision\":4",
        "\"expected_slot_revision\":\"4\"",
        1,
    );
    assert!(matches!(
        Revision3VoiceTakeSelectionRequestV1::from_json(&wrong_type),
        Err(Revision3VoiceTakeSelectionRequestJsonErrorV1::InvalidJson(
            _
        ))
    ));

    let oversized_json = "x".repeat(MAX_REVISION3_VOICE_TAKE_SELECTION_REQUEST_JSON_BYTES_V1 + 1);
    assert!(matches!(
        Revision3VoiceTakeSelectionRequestV1::from_json(&oversized_json),
        Err(Revision3VoiceTakeSelectionRequestJsonErrorV1::InputTooLarge { .. })
    ));
    let mut oversized_request = request;
    oversized_request.expected_loc_id =
        "X".repeat(MAX_REVISION3_VOICE_TAKE_SELECTION_REQUEST_JSON_BYTES_V1);
    assert!(matches!(
        oversized_request.to_canonical_json(),
        Err(Revision3VoiceTakeSelectionRequestJsonErrorV1::InputTooLarge { .. })
    ));
}

#[test]
fn candidate_too_large_is_distinct_from_an_oversized_invalid_basis() {
    let mut project = basis();
    slot_mut(&mut project, id(3)).selected = None;
    project.meta.author.clear();
    let unpadded = project.to_canonical_json().unwrap();
    project.meta.author = "x".repeat(MAX_PROJECT_JSON_BYTES - unpadded.len());
    let canonical_basis = project.to_canonical_json().unwrap();
    assert_eq!(canonical_basis.len(), MAX_PROJECT_JSON_BYTES);
    let request = request(&project, None, Some(id(5)));
    let request_json = request.to_canonical_json().unwrap();

    let result = apply_revision3_voice_take_selection_transaction_v1(
        &basis_head(),
        &canonical_basis,
        &request_json,
    )
    .unwrap();
    let Revision3VoiceTakeSelectionEvaluationV1::Rejected(rejection) = result else {
        panic!("selection must not emit an oversized candidate")
    };
    assert!(matches!(
        rejection.conflict,
        Revision3VoiceTakeSelectionConflictV1::CandidateTooLarge {
            actual,
            limit: MAX_PROJECT_JSON_BYTES,
        } if actual > MAX_PROJECT_JSON_BYTES
    ));

    let mut oversized_project = project;
    oversized_project.meta.author.push('x');
    let oversized_basis = serde_json::to_string(&oversized_project).unwrap();
    assert_eq!(oversized_basis.len(), MAX_PROJECT_JSON_BYTES + 1);
    assert!(matches!(
        apply_revision3_voice_take_selection_transaction_v1(
            &basis_head(),
            &oversized_basis,
            &request_json,
        ),
        Err(Revision3VoiceTakeSelectionErrorV1::InvalidProject(
            ProjectRevision3JsonError::InputTooLarge {
                actual,
                limit: MAX_PROJECT_JSON_BYTES,
            }
        )) if actual == MAX_PROJECT_JSON_BYTES + 1
    ));
}
