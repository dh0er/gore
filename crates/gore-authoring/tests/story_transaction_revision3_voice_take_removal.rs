use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::model_revision3::{
    DialogLine, Entity, EntityKind, EntityPayload, LocalizationEntry, OggCodec, OggMetadata,
    OriginRef, SchemaRevisionV3, TypedRef, VoiceMemberProof, VoiceOperation, VoiceSlot, VoiceTake,
    VoiceTakeStatus, VoiceTarget, VoiceTargetResolution,
};
use gore_authoring::{
    apply_revision3_voice_take_removal_transaction_v1, ArchiveSeal, AssetMeta, AssetRef,
    AssetStoreIndex, ContentSeal, EntityId, FormatV2, GameGenerationAnchor, LocaleCode, ProjectId,
    ProjectMeta, ProjectRevision3, Revision3VoiceTakeRemovalBuildStatusV1,
    Revision3VoiceTakeRemovalConflictV1, Revision3VoiceTakeRemovalEvaluationV1,
    Revision3VoiceTakeRemovalRequestJsonErrorV1, Revision3VoiceTakeRemovalRequestV1,
    Revision3VoiceTakeRemovalRuntimeStatusV1, Sha256Digest, WorkingHead, WorkingStoreFormat,
    MAX_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_JSON_BYTES_V1,
};

const LOC_ID_ONE: &str = "GRD_263_ASGHAN_OPEN_INFO_06_02";
const LOC_ID_TWO: &str = "STT_302_VIPER_GREET_INFO_11_02";

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
        importer: "voice-removal-tests".to_owned(),
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

fn take_entity(take_id: EntityId, revision: u64, asset: AssetRef, name: &str) -> Entity {
    Entity {
        id: take_id,
        display_name: name.to_owned(),
        origin: origin(take_id.as_bytes()[0]),
        revision,
        payload: EntityPayload::VoiceTake(VoiceTake {
            locale: locale(),
            asset,
            ogg: OggMetadata {
                codec: OggCodec::Vorbis,
                channels: 1,
                sample_rate: 48_000,
                pages: 3,
                logical_streams: 1,
            },
            status: VoiceTakeStatus::Approved,
        }),
    }
}

/// Two closed lines. Take 5 is shared; takes 10 and 11 intentionally share one digest.
fn basis() -> ProjectRevision3 {
    let pid = project_id(0x10);
    let locale = locale();
    let take_a_asset = asset(0x41, "asghan-a.ogg");
    let shared_asset = asset(0x42, "shared-b.ogg");
    let same_digest_a = asset(0x44, "same-digest-a.ogg");
    let same_digest_b = AssetRef {
        logical_name: "same-digest-b.ogg".to_owned(),
        ..same_digest_a.clone()
    };
    ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id: pid,
        revision: 11,
        meta: ProjectMeta {
            name: "VoiceRemoval".to_owned(),
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
                        localization: TypedRef::new(pid, id(1), EntityKind::LocalizationEntry),
                        speaker_hint: Some("Asghan".to_owned()),
                        voice_slots: BTreeMap::from([(
                            locale.clone(),
                            TypedRef::new(pid, id(3), EntityKind::VoiceSlot),
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
                            TypedRef::new(pid, id(4), EntityKind::VoiceTake),
                            TypedRef::new(pid, id(5), EntityKind::VoiceTake),
                            TypedRef::new(pid, id(10), EntityKind::VoiceTake),
                        ],
                        selected: Some(TypedRef::new(pid, id(4), EntityKind::VoiceTake)),
                    }),
                },
            ),
            (
                id(4),
                take_entity(id(4), 41, take_a_asset.clone(), "Asghan take A"),
            ),
            (
                id(5),
                take_entity(id(5), 42, shared_asset.clone(), "Shared take B"),
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
                        localization: TypedRef::new(pid, id(6), EntityKind::LocalizationEntry),
                        speaker_hint: Some("Viper".to_owned()),
                        voice_slots: BTreeMap::from([(
                            locale.clone(),
                            TypedRef::new(pid, id(8), EntityKind::VoiceSlot),
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
                            TypedRef::new(pid, id(5), EntityKind::VoiceTake),
                            TypedRef::new(pid, id(11), EntityKind::VoiceTake),
                        ],
                        selected: Some(TypedRef::new(pid, id(5), EntityKind::VoiceTake)),
                    }),
                },
            ),
            (
                id(10),
                take_entity(id(10), 10, same_digest_a.clone(), "Same digest A"),
            ),
            (
                id(11),
                take_entity(id(11), 11, same_digest_b, "Same digest B"),
            ),
        ]),
        asset_store: AssetStoreIndex {
            assets: BTreeMap::from([
                (
                    take_a_asset.sha256,
                    AssetMeta {
                        byte_len: take_a_asset.byte_len,
                        media_type: "audio/ogg".to_owned(),
                    },
                ),
                (
                    shared_asset.sha256,
                    AssetMeta {
                        byte_len: shared_asset.byte_len,
                        media_type: "audio/ogg".to_owned(),
                    },
                ),
                (
                    same_digest_a.sha256,
                    AssetMeta {
                        byte_len: same_digest_a.byte_len,
                        media_type: "audio/ogg".to_owned(),
                    },
                ),
                (
                    digest(0xee),
                    AssetMeta {
                        byte_len: 17,
                        media_type: "application/octet-stream".to_owned(),
                    },
                ),
            ]),
        },
    }
}

fn request(project: &ProjectRevision3, take_id: EntityId) -> Revision3VoiceTakeRemovalRequestV1 {
    Revision3VoiceTakeRemovalRequestV1 {
        expected_head: basis_head(),
        expected_project_id: project.project_id,
        expected_revision: project.revision,
        expected_target: project.target.clone(),
        line_id: id(2),
        localization_id: id(1),
        expected_loc_id: LOC_ID_ONE.to_owned(),
        locale: locale(),
        slot_id: id(3),
        expected_slot_revision: project.entities[&id(3)].revision,
        take_id,
        expected_take_revision: project.entities[&take_id].revision,
        expected_selected_take_id: slot(project, id(3)).selected.as_ref().map(|value| value.id),
    }
}

fn apply(
    project: &ProjectRevision3,
    request: &Revision3VoiceTakeRemovalRequestV1,
) -> Revision3VoiceTakeRemovalEvaluationV1 {
    apply_with_head(project, request, &basis_head())
}

fn apply_with_head(
    project: &ProjectRevision3,
    request: &Revision3VoiceTakeRemovalRequestV1,
    exact_head: &WorkingHead,
) -> Revision3VoiceTakeRemovalEvaluationV1 {
    apply_revision3_voice_take_removal_transaction_v1(
        exact_head,
        &project.to_canonical_json().unwrap(),
        &request.to_canonical_json().unwrap(),
    )
    .unwrap()
}

fn conflict(
    project: &ProjectRevision3,
    request: &Revision3VoiceTakeRemovalRequestV1,
) -> Revision3VoiceTakeRemovalConflictV1 {
    let before = project.clone();
    let canonical_before = project.to_canonical_json().unwrap();
    let Revision3VoiceTakeRemovalEvaluationV1::Rejected(rejection) = apply(project, request) else {
        panic!("expected rejected Voice take removal")
    };
    assert_eq!(project, &before, "rejection mutated the caller project");
    assert_eq!(project.to_canonical_json().unwrap(), canonical_before);
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

#[test]
fn unique_nonselected_take_is_unlinked_and_entity_removed_without_digest_collateral() {
    let project = basis();
    let asset_store = project.asset_store.clone();
    let other_same_digest = project.entities[&id(11)].clone();
    let request = request(&project, id(10));
    let Revision3VoiceTakeRemovalEvaluationV1::Applied(outcome) = apply(&project, &request) else {
        panic!("expected applied removal")
    };

    assert_eq!(outcome.project.revision, project.revision + 1);
    assert_eq!(outcome.slot_revision, 5);
    assert_eq!(outcome.take_revision, 10);
    assert_eq!(outcome.previous_selected_take_id, Some(id(4)));
    assert!(!outcome.selection_cleared);
    assert!(outcome.take_entity_removed);
    assert_eq!(outcome.remaining_candidate_count, 2);
    assert_eq!(
        outcome.build_status,
        Revision3VoiceTakeRemovalBuildStatusV1::Blocked
    );
    assert_eq!(
        outcome.runtime_status,
        Revision3VoiceTakeRemovalRuntimeStatusV1::RuntimeUnqualified
    );
    assert_eq!(&outcome.project.asset_store, &asset_store);
    assert!(!outcome.project.entities.contains_key(&id(10)));
    assert_eq!(outcome.project.entities[&id(11)], other_same_digest);
    assert_eq!(
        slot(&outcome.project, id(3))
            .candidates
            .iter()
            .map(|value| value.id)
            .collect::<Vec<_>>(),
        vec![id(4), id(5)]
    );
    assert_eq!(
        slot(&outcome.project, id(3)).selected.as_ref().unwrap().id,
        id(4)
    );
    assert_eq!(
        ProjectRevision3::from_json(&outcome.canonical_project_json).unwrap(),
        outcome.project
    );
}

#[test]
fn selected_take_removal_clears_selection_atomically_and_preserves_every_other_value() {
    let project = basis();
    let request = request(&project, id(4));
    let Revision3VoiceTakeRemovalEvaluationV1::Applied(outcome) = apply(&project, &request) else {
        panic!("expected applied removal")
    };

    let mut expected = project.clone();
    expected.revision += 1;
    expected.entities.get_mut(&id(3)).unwrap().revision += 1;
    slot_mut(&mut expected, id(3)).candidates.remove(0);
    slot_mut(&mut expected, id(3)).selected = None;
    expected.entities.remove(&id(4));
    assert_eq!(outcome.project, expected);
    assert!(outcome.selection_cleared);
    assert_eq!(outcome.previous_selected_take_id, Some(id(4)));
}

#[test]
fn shared_take_is_detached_here_but_retained_byte_exact_when_selected_elsewhere() {
    let project = basis();
    let shared_before = project.entities[&id(5)].clone();
    let other_slot_before = project.entities[&id(8)].clone();
    let request = request(&project, id(5));
    let Revision3VoiceTakeRemovalEvaluationV1::Applied(outcome) = apply(&project, &request) else {
        panic!("expected applied removal")
    };

    assert!(!outcome.take_entity_removed);
    assert_eq!(outcome.project.entities[&id(5)], shared_before);
    assert_eq!(outcome.project.entities[&id(8)], other_slot_before);
    assert_eq!(
        slot(&outcome.project, id(8)).selected.as_ref().unwrap().id,
        id(5)
    );
    assert_eq!(
        slot(&outcome.project, id(3))
            .candidates
            .iter()
            .map(|value| value.id)
            .collect::<Vec<_>>(),
        vec![id(4), id(10)]
    );
}

#[test]
fn final_candidate_can_leave_the_slot_empty_without_changing_target_or_origin() {
    let mut project = basis();
    let slot_before = project.entities[&id(3)].clone();
    let target_before = slot(&project, id(3)).target_resolution.clone();
    slot_mut(&mut project, id(3)).candidates = vec![TypedRef::new(
        project.project_id,
        id(4),
        EntityKind::VoiceTake,
    )];
    let request = request(&project, id(4));
    let Revision3VoiceTakeRemovalEvaluationV1::Applied(outcome) = apply(&project, &request) else {
        panic!("expected applied removal")
    };

    let resulting_slot_entity = &outcome.project.entities[&id(3)];
    assert!(slot(&outcome.project, id(3)).candidates.is_empty());
    assert!(slot(&outcome.project, id(3)).selected.is_none());
    assert_eq!(
        slot(&outcome.project, id(3)).target_resolution,
        target_before
    );
    assert_eq!(resulting_slot_entity.origin, slot_before.origin);
    assert_eq!(resulting_slot_entity.display_name, slot_before.display_name);
    assert_eq!(outcome.remaining_candidate_count, 0);
}

#[test]
fn foreign_project_same_entity_id_backlink_is_ignored() {
    let mut project = basis();
    project.entities.get_mut(&id(11)).unwrap().origin = OriginRef::Generated {
        generator_id: "foreign-fixture".to_owned(),
        generator_version: 1,
        owner: TypedRef::new(project_id(0x99), id(10), EntityKind::VoiceTake),
    };
    let request = request(&project, id(10));
    let Revision3VoiceTakeRemovalEvaluationV1::Applied(outcome) = apply(&project, &request) else {
        panic!("foreign same-ID reference must not block local removal")
    };
    assert!(outcome.take_entity_removed);
}

#[test]
fn unexpected_and_kind_mismatched_local_backlinks_fail_closed() {
    for expected_kind in [EntityKind::VoiceTake, EntityKind::LocalizationEntry] {
        let mut project = basis();
        project.entities.get_mut(&id(11)).unwrap().origin = OriginRef::Generated {
            generator_id: "unsafe-fixture".to_owned(),
            generator_version: 1,
            owner: TypedRef::new(project.project_id, id(10), expected_kind),
        };
        let request = request(&project, id(10));
        assert!(matches!(
            conflict(&project, &request),
            Revision3VoiceTakeRemovalConflictV1::InvalidLocalBacklink {
                take,
                source_entity,
                ..
            } if take == id(10) && source_entity == id(11)
        ));
    }
}

#[test]
fn stale_head_project_slot_take_and_selection_bindings_are_rejected() {
    let project = basis();

    let head_request = request(&project, id(10));
    let Revision3VoiceTakeRemovalEvaluationV1::Rejected(rejection) =
        apply_with_head(&project, &head_request, &head(0x32))
    else {
        panic!("expected head rejection")
    };
    assert_eq!(
        rejection.conflict,
        Revision3VoiceTakeRemovalConflictV1::CurrentHeadMismatch
    );

    let mut stale = request(&project, id(10));
    stale.expected_revision -= 1;
    assert!(matches!(
        conflict(&project, &stale),
        Revision3VoiceTakeRemovalConflictV1::ProjectRevisionConflict { .. }
    ));
    let mut stale = request(&project, id(10));
    stale.expected_slot_revision -= 1;
    assert!(matches!(
        conflict(&project, &stale),
        Revision3VoiceTakeRemovalConflictV1::VoiceSlotRevisionConflict { .. }
    ));
    let mut stale = request(&project, id(10));
    stale.expected_take_revision -= 1;
    assert!(matches!(
        conflict(&project, &stale),
        Revision3VoiceTakeRemovalConflictV1::VoiceTakeRevisionConflict { .. }
    ));
    let mut stale = request(&project, id(10));
    stale.expected_selected_take_id = None;
    assert!(matches!(
        conflict(&project, &stale),
        Revision3VoiceTakeRemovalConflictV1::CurrentSelectionMismatch { .. }
    ));
}

#[test]
fn exact_line_localization_slot_and_target_are_bound() {
    let project = basis();
    let mut invalid = request(&project, id(10));
    invalid.localization_id = id(6);
    assert!(matches!(
        conflict(&project, &invalid),
        Revision3VoiceTakeRemovalConflictV1::InvalidLocalizationReference { .. }
    ));
    let mut invalid = request(&project, id(10));
    invalid.expected_loc_id = LOC_ID_TWO.to_owned();
    assert!(matches!(
        conflict(&project, &invalid),
        Revision3VoiceTakeRemovalConflictV1::LocalizationIdentityMismatch { .. }
    ));
    let mut invalid = request(&project, id(10));
    invalid.slot_id = id(8);
    assert!(matches!(
        conflict(&project, &invalid),
        Revision3VoiceTakeRemovalConflictV1::VoiceSlotIdentityMismatch { .. }
    ));
    let mut invalid = request(&project, id(10));
    invalid.expected_target.executable.byte_len += 1;
    assert_eq!(
        conflict(&project, &invalid),
        Revision3VoiceTakeRemovalConflictV1::ProjectTargetMismatch
    );
}

#[test]
fn missing_candidate_and_revision_overflows_are_typed_conflicts() {
    let project = basis();
    let absent_request = request(&project, id(11));
    assert!(matches!(
        conflict(&project, &absent_request),
        Revision3VoiceTakeRemovalConflictV1::VoiceTakeNotExactCandidate { .. }
    ));

    let mut overflow = basis();
    overflow.revision = u64::MAX;
    let project_overflow_request = request(&overflow, id(10));
    assert_eq!(
        conflict(&overflow, &project_overflow_request),
        Revision3VoiceTakeRemovalConflictV1::ProjectRevisionOverflow
    );

    let mut overflow = basis();
    overflow.entities.get_mut(&id(3)).unwrap().revision = u64::MAX;
    let slot_overflow_request = request(&overflow, id(10));
    assert!(matches!(
        conflict(&overflow, &slot_overflow_request),
        Revision3VoiceTakeRemovalConflictV1::VoiceSlotRevisionOverflow { slot } if slot == id(3)
    ));
}

#[test]
fn request_json_is_exact_canonical_duplicate_free_and_bounded() {
    let project = basis();
    let request = request(&project, id(10));
    let canonical = request.to_canonical_json().unwrap();
    assert_eq!(
        Revision3VoiceTakeRemovalRequestV1::from_json(&canonical).unwrap(),
        request
    );
    assert!(
        canonical.find("\"expected_head\"").unwrap()
            < canonical.find("\"expected_project_id\"").unwrap()
    );
    assert!(
        canonical.find("\"line_id\"").unwrap() < canonical.find("\"localization_id\"").unwrap()
    );
    assert!(
        canonical.find("\"localization_id\"").unwrap()
            < canonical.find("\"expected_loc_id\"").unwrap()
    );
    assert!(
        canonical.find("\"take_id\"").unwrap()
            < canonical.find("\"expected_take_revision\"").unwrap()
    );
    assert!(matches!(
        Revision3VoiceTakeRemovalRequestV1::from_json(&format!(" {canonical}")),
        Err(Revision3VoiceTakeRemovalRequestJsonErrorV1::NonCanonicalJson)
    ));
    let duplicate = canonical.replacen(
        "\"expected_revision\":11,",
        "\"expected_revision\":11,\"expected_revision\":11,",
        1,
    );
    assert!(matches!(
        Revision3VoiceTakeRemovalRequestV1::from_json(&duplicate),
        Err(Revision3VoiceTakeRemovalRequestJsonErrorV1::InvalidJson(_))
    ));
    let oversized = "x".repeat(MAX_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_JSON_BYTES_V1 + 1);
    assert!(matches!(
        Revision3VoiceTakeRemovalRequestV1::from_json(&oversized),
        Err(Revision3VoiceTakeRemovalRequestJsonErrorV1::InputTooLarge { .. })
    ));
}
