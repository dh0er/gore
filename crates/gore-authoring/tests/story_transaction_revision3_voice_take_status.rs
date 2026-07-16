use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::model_revision3::{
    DialogLine, Entity, EntityKind, EntityPayload, LocalizationEntry, OggCodec, OggMetadata,
    OriginRef, SchemaRevisionV3, TypedRef, VoiceSlot, VoiceTake, VoiceTakeStatus,
    VoiceTargetResolution,
};
use gore_authoring::{
    apply_revision3_voice_take_status_edit_transaction_v1, AssetMeta, AssetRef, AssetStoreIndex,
    ContentSeal, EntityId, FormatV2, GameGenerationAnchor, LocaleCode, ProjectId, ProjectMeta,
    ProjectRevision3, ProjectRevision3JsonError, Revision3VoiceTakeStatusEditBuildStatusV1,
    Revision3VoiceTakeStatusEditConflictV1, Revision3VoiceTakeStatusEditErrorV1,
    Revision3VoiceTakeStatusEditEvaluationV1, Revision3VoiceTakeStatusEditRequestJsonErrorV1,
    Revision3VoiceTakeStatusEditRequestV1, Revision3VoiceTakeStatusEditRuntimeStatusV1,
    Sha256Digest, WorkingHead, WorkingStoreFormat, MAX_PROJECT_JSON_BYTES,
    MAX_REVISION3_VOICE_TAKE_STATUS_EDIT_REQUEST_JSON_BYTES_V1,
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
        importer: "voice-status-tests".to_owned(),
        source_seal: seal(tag, 100),
        external_identity: None,
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
    revision: u64,
    status: VoiceTakeStatus,
) -> Entity {
    Entity {
        id: take_id,
        display_name: name.to_owned(),
        origin: origin(tag),
        revision,
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

/// Two complete line/slot graphs. Every take is uniquely retained in the valid basis.
fn basis() -> ProjectRevision3 {
    let project_id = project_id();
    let locale = locale();
    let take_a = asset(0x41, "asghan-a.ogg");
    let take_b = asset(0x42, "asghan-b.ogg");
    let take_c = asset(0x43, "viper-c.ogg");
    ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id,
        revision: 11,
        meta: ProjectMeta {
            name: "VoiceTakeStatus".to_owned(),
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
                        target_resolution: VoiceTargetResolution::Unresolved,
                        candidates: vec![
                            TypedRef::new(project_id, id(4), EntityKind::VoiceTake),
                            TypedRef::new(project_id, id(5), EntityKind::VoiceTake),
                        ],
                        selected: Some(TypedRef::new(project_id, id(5), EntityKind::VoiceTake)),
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
                    6,
                    VoiceTakeStatus::Recorded,
                ),
            ),
            (
                id(5),
                take_entity(
                    id(5),
                    0x42,
                    "Asghan take B",
                    "asghan-b.ogg",
                    7,
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
                        target_resolution: VoiceTargetResolution::Unresolved,
                        candidates: vec![TypedRef::new(project_id, id(9), EntityKind::VoiceTake)],
                        selected: Some(TypedRef::new(project_id, id(9), EntityKind::VoiceTake)),
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
                    9,
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
    take_id: EntityId,
    expected_status: VoiceTakeStatus,
    desired_status: VoiceTakeStatus,
) -> Revision3VoiceTakeStatusEditRequestV1 {
    Revision3VoiceTakeStatusEditRequestV1 {
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
        expected_status,
        desired_status,
    }
}

fn apply(
    project: &ProjectRevision3,
    request: &Revision3VoiceTakeStatusEditRequestV1,
) -> Revision3VoiceTakeStatusEditEvaluationV1 {
    apply_with_head(project, request, &basis_head())
}

fn apply_with_head(
    project: &ProjectRevision3,
    request: &Revision3VoiceTakeStatusEditRequestV1,
    exact_head: &WorkingHead,
) -> Revision3VoiceTakeStatusEditEvaluationV1 {
    apply_revision3_voice_take_status_edit_transaction_v1(
        exact_head,
        &project.to_canonical_json().unwrap(),
        &request.to_canonical_json().unwrap(),
    )
    .unwrap()
}

fn conflict(
    project: &ProjectRevision3,
    request: &Revision3VoiceTakeStatusEditRequestV1,
) -> Revision3VoiceTakeStatusEditConflictV1 {
    let before = project.clone();
    let canonical_before = project.to_canonical_json().unwrap();
    let Revision3VoiceTakeStatusEditEvaluationV1::Rejected(rejection) = apply(project, request)
    else {
        panic!("expected rejected Voice take status edit")
    };
    assert_eq!(project, &before, "rejection mutated the caller's project");
    assert_eq!(
        project.to_canonical_json().unwrap(),
        canonical_before,
        "rejection changed the caller's canonical bytes"
    );
    rejection.conflict
}

fn take(project: &ProjectRevision3, take_id: EntityId) -> &VoiceTake {
    let EntityPayload::VoiceTake(take) = &project.entities[&take_id].payload else {
        panic!("expected VoiceTake")
    };
    take
}

fn take_mut(project: &mut ProjectRevision3, take_id: EntityId) -> &mut VoiceTake {
    let EntityPayload::VoiceTake(take) = &mut project.entities.get_mut(&take_id).unwrap().payload
    else {
        panic!("expected VoiceTake")
    };
    take
}

fn slot_mut(project: &mut ProjectRevision3, slot_id: EntityId) -> &mut VoiceSlot {
    let EntityPayload::VoiceSlot(slot) = &mut project.entities.get_mut(&slot_id).unwrap().payload
    else {
        panic!("expected VoiceSlot")
    };
    slot
}

fn line_mut(project: &mut ProjectRevision3, line_id: EntityId) -> &mut DialogLine {
    let EntityPayload::DialogLine(line) = &mut project.entities.get_mut(&line_id).unwrap().payload
    else {
        panic!("expected DialogLine")
    };
    line
}

#[test]
fn status_edit_changes_only_project_and_take_revision_plus_status() {
    let project = basis();
    let request = request(
        &project,
        id(4),
        VoiceTakeStatus::Recorded,
        VoiceTakeStatus::Reviewed,
    );
    let Revision3VoiceTakeStatusEditEvaluationV1::Applied(outcome) = apply(&project, &request)
    else {
        panic!("expected applied Voice take status edit")
    };

    let mut expected = project.clone();
    expected.revision += 1;
    expected.entities.get_mut(&id(4)).unwrap().revision += 1;
    take_mut(&mut expected, id(4)).status = VoiceTakeStatus::Reviewed;

    assert_eq!(outcome.project, expected);
    assert_eq!(project.revision, 11, "pure transaction changed its input");
    assert_eq!(outcome.basis_head, basis_head());
    assert_eq!(outcome.line_id, id(2));
    assert_eq!(outcome.localization_id, id(1));
    assert_eq!(outcome.slot_id, id(3));
    assert_eq!(outcome.slot_revision, 4);
    assert_eq!(outcome.take_id, id(4));
    assert_eq!(outcome.take_revision, 7);
    assert_eq!(outcome.locale, locale());
    assert_eq!(outcome.loc_id, LOC_ID_ONE);
    assert_eq!(outcome.previous_status, VoiceTakeStatus::Recorded);
    assert_eq!(outcome.status, VoiceTakeStatus::Reviewed);
    assert_eq!(
        outcome.build_status,
        Revision3VoiceTakeStatusEditBuildStatusV1::Blocked
    );
    assert_eq!(
        outcome.runtime_status,
        Revision3VoiceTakeStatusEditRuntimeStatusV1::RuntimeUnqualified
    );
    assert_eq!(
        ProjectRevision3::from_json(&outcome.canonical_project_json).unwrap(),
        outcome.project
    );
    assert_eq!(outcome.project.asset_store, project.asset_store);
    for entity_id in [id(1), id(2), id(3), id(5), id(6), id(7), id(8), id(9)] {
        assert_eq!(
            outcome.project.entities[&entity_id], project.entities[&entity_id],
            "unrelated entity {entity_id} changed"
        );
    }
}

#[test]
fn every_nonselected_status_transition_is_author_managed_and_exact() {
    for current in [
        VoiceTakeStatus::Draft,
        VoiceTakeStatus::Recorded,
        VoiceTakeStatus::Reviewed,
        VoiceTakeStatus::Approved,
    ] {
        for desired in [
            VoiceTakeStatus::Draft,
            VoiceTakeStatus::Recorded,
            VoiceTakeStatus::Reviewed,
            VoiceTakeStatus::Approved,
        ] {
            if current == desired {
                continue;
            }
            let mut project = basis();
            take_mut(&mut project, id(4)).status = current;
            let request = request(&project, id(4), current, desired);
            let Revision3VoiceTakeStatusEditEvaluationV1::Applied(outcome) =
                apply(&project, &request)
            else {
                panic!("expected {current:?} -> {desired:?} to apply")
            };
            assert_eq!(take(&outcome.project, id(4)).status, desired);
        }
    }
}

#[test]
fn selected_approved_take_cannot_be_demoted_until_selection_is_cleared() {
    let project = basis();
    for desired in [
        VoiceTakeStatus::Draft,
        VoiceTakeStatus::Recorded,
        VoiceTakeStatus::Reviewed,
    ] {
        let request = request(&project, id(5), VoiceTakeStatus::Approved, desired);
        assert_eq!(
            conflict(&project, &request),
            Revision3VoiceTakeStatusEditConflictV1::SelectedTakeCannotBecomeUnapproved {
                take: id(5),
            }
        );
    }

    let mut cleared = project;
    slot_mut(&mut cleared, id(3)).selected = None;
    let request = request(
        &cleared,
        id(5),
        VoiceTakeStatus::Approved,
        VoiceTakeStatus::Recorded,
    );
    let Revision3VoiceTakeStatusEditEvaluationV1::Applied(outcome) = apply(&cleared, &request)
    else {
        panic!("cleared take should be demotable")
    };
    assert_eq!(
        take(&outcome.project, id(5)).status,
        VoiceTakeStatus::Recorded
    );
}

#[test]
fn shared_take_and_noncandidate_fail_closed() {
    let mut shared = basis();
    slot_mut(&mut shared, id(8)).candidates.push(TypedRef::new(
        project_id(),
        id(4),
        EntityKind::VoiceTake,
    ));
    let shared_request = request(
        &shared,
        id(4),
        VoiceTakeStatus::Recorded,
        VoiceTakeStatus::Reviewed,
    );
    assert_eq!(
        conflict(&shared, &shared_request),
        Revision3VoiceTakeStatusEditConflictV1::SharedVoiceTake { take: id(4) }
    );

    let project = basis();
    let request = request(
        &project,
        id(9),
        VoiceTakeStatus::Approved,
        VoiceTakeStatus::Reviewed,
    );
    assert_eq!(
        conflict(&project, &request),
        Revision3VoiceTakeStatusEditConflictV1::VoiceTakeNotCandidate {
            slot: id(3),
            take: id(9),
        }
    );
}

#[test]
fn malformed_graph_matrix_fails_at_the_closed_project_boundary() {
    #[derive(Debug, Clone, Copy)]
    enum Damage {
        SharedSlotOwner,
        DuplicateCandidate,
        WrongLocalizationKind,
        MissingLocalization,
        WrongSlotKind,
        MissingSlot,
        WrongTakeKind,
        MissingTake,
        TakeLocaleDrift,
    }

    for damage in [
        Damage::SharedSlotOwner,
        Damage::DuplicateCandidate,
        Damage::WrongLocalizationKind,
        Damage::MissingLocalization,
        Damage::WrongSlotKind,
        Damage::MissingSlot,
        Damage::WrongTakeKind,
        Damage::MissingTake,
        Damage::TakeLocaleDrift,
    ] {
        let valid = basis();
        let request = request(
            &valid,
            id(4),
            VoiceTakeStatus::Recorded,
            VoiceTakeStatus::Reviewed,
        );
        let mut damaged = valid;
        match damage {
            Damage::SharedSlotOwner => {
                line_mut(&mut damaged, id(7)).voice_slots.insert(
                    locale(),
                    TypedRef::new(project_id(), id(3), EntityKind::VoiceSlot),
                );
            }
            Damage::DuplicateCandidate => {
                slot_mut(&mut damaged, id(3)).candidates.push(TypedRef::new(
                    project_id(),
                    id(4),
                    EntityKind::VoiceTake,
                ));
            }
            Damage::WrongLocalizationKind => {
                line_mut(&mut damaged, id(2)).localization =
                    TypedRef::new(project_id(), id(4), EntityKind::LocalizationEntry);
            }
            Damage::MissingLocalization => {
                line_mut(&mut damaged, id(2)).localization =
                    TypedRef::new(project_id(), id(0xa0), EntityKind::LocalizationEntry);
            }
            Damage::WrongSlotKind => {
                line_mut(&mut damaged, id(2)).voice_slots.insert(
                    locale(),
                    TypedRef::new(project_id(), id(4), EntityKind::VoiceSlot),
                );
            }
            Damage::MissingSlot => {
                line_mut(&mut damaged, id(2)).voice_slots.insert(
                    locale(),
                    TypedRef::new(project_id(), id(0xa1), EntityKind::VoiceSlot),
                );
            }
            Damage::WrongTakeKind => {
                slot_mut(&mut damaged, id(3)).candidates[0] =
                    TypedRef::new(project_id(), id(1), EntityKind::VoiceTake);
            }
            Damage::MissingTake => {
                slot_mut(&mut damaged, id(3)).candidates[0] =
                    TypedRef::new(project_id(), id(0xa2), EntityKind::VoiceTake);
            }
            Damage::TakeLocaleDrift => {
                let english: LocaleCode = "en".parse().unwrap();
                damaged.authoring_locales.insert(english.clone());
                take_mut(&mut damaged, id(4)).locale = english;
            }
        }

        // Invalid graphs cannot pass `ProjectRevision3::to_canonical_json`; serialize the wire
        // shape directly to prove the transaction's first boundary rejects it before any delta.
        let damaged_json = serde_json::to_string(&damaged).unwrap();
        assert!(
            matches!(
                apply_revision3_voice_take_status_edit_transaction_v1(
                    &basis_head(),
                    &damaged_json,
                    &request.to_canonical_json().unwrap(),
                ),
                Err(Revision3VoiceTakeStatusEditErrorV1::InvalidProject(_))
            ),
            "malformed {damage:?} graph escaped the closed project boundary"
        );
    }
}

#[test]
fn exact_head_project_target_graph_revisions_and_previous_status_are_bound() {
    let project = basis();
    let base = request(
        &project,
        id(4),
        VoiceTakeStatus::Recorded,
        VoiceTakeStatus::Reviewed,
    );

    let Revision3VoiceTakeStatusEditEvaluationV1::Rejected(rejection) =
        apply_with_head(&project, &base, &head(0x32))
    else {
        panic!("expected stale head rejection")
    };
    assert_eq!(
        rejection.conflict,
        Revision3VoiceTakeStatusEditConflictV1::CurrentHeadMismatch
    );

    let mut request = base.clone();
    request.expected_project_id = ProjectId::from_bytes([0x99; 16]);
    assert!(matches!(
        conflict(&project, &request),
        Revision3VoiceTakeStatusEditConflictV1::ProjectIdentityMismatch { .. }
    ));

    request = base.clone();
    request.expected_revision += 1;
    assert_eq!(
        conflict(&project, &request),
        Revision3VoiceTakeStatusEditConflictV1::ProjectRevisionConflict {
            expected: 12,
            actual: 11,
        }
    );

    request = base.clone();
    request.expected_target.executable.sha256 = digest(0x99);
    assert_eq!(
        conflict(&project, &request),
        Revision3VoiceTakeStatusEditConflictV1::ProjectTargetMismatch
    );

    request = base.clone();
    request.localization_id = id(6);
    assert_eq!(
        conflict(&project, &request),
        Revision3VoiceTakeStatusEditConflictV1::LocalizationReferenceMismatch {
            line: id(2),
            localization: id(6),
        }
    );

    request = base.clone();
    request.expected_loc_id = LOC_ID_TWO.to_owned();
    assert_eq!(
        conflict(&project, &request),
        Revision3VoiceTakeStatusEditConflictV1::LocalizationIdentityMismatch {
            expected: LOC_ID_TWO.to_owned(),
            actual: LOC_ID_ONE.to_owned(),
        }
    );

    request = base.clone();
    request.locale = "en".parse().unwrap();
    assert_eq!(
        conflict(&project, &request),
        Revision3VoiceTakeStatusEditConflictV1::VoiceSlotIdentityMismatch { slot: id(3) }
    );

    request = base.clone();
    request.expected_slot_revision += 1;
    assert_eq!(
        conflict(&project, &request),
        Revision3VoiceTakeStatusEditConflictV1::VoiceSlotRevisionConflict {
            expected: 5,
            actual: 4,
        }
    );

    request = base.clone();
    request.expected_take_revision += 1;
    assert_eq!(
        conflict(&project, &request),
        Revision3VoiceTakeStatusEditConflictV1::VoiceTakeRevisionConflict {
            expected: 7,
            actual: 6,
        }
    );

    request = base;
    request.expected_status = VoiceTakeStatus::Draft;
    assert_eq!(
        conflict(&project, &request),
        Revision3VoiceTakeStatusEditConflictV1::CurrentStatusMismatch {
            expected: VoiceTakeStatus::Draft,
            actual: VoiceTakeStatus::Recorded,
        }
    );
}

#[test]
fn invalid_identity_loc_id_noop_and_revision_overflows_are_rejected() {
    let project = basis();
    let base = request(
        &project,
        id(4),
        VoiceTakeStatus::Recorded,
        VoiceTakeStatus::Reviewed,
    );

    for invalid in [EntityId::from_bytes([0; 16]), id(2), id(1), id(3)] {
        let mut request = base.clone();
        request.take_id = invalid;
        assert_eq!(
            conflict(&project, &request),
            Revision3VoiceTakeStatusEditConflictV1::InvalidEntityIdentity
        );
    }

    let mut edit = base.clone();
    edit.expected_loc_id = "../unsafe".to_owned();
    assert_eq!(
        conflict(&project, &edit),
        Revision3VoiceTakeStatusEditConflictV1::InvalidExpectedLocId
    );

    edit = base.clone();
    edit.desired_status = VoiceTakeStatus::Recorded;
    assert_eq!(
        conflict(&project, &edit),
        Revision3VoiceTakeStatusEditConflictV1::NoChanges
    );

    let mut project_overflow = project.clone();
    project_overflow.revision = u64::MAX;
    let overflow_request = request(
        &project_overflow,
        id(4),
        VoiceTakeStatus::Recorded,
        VoiceTakeStatus::Reviewed,
    );
    assert_eq!(
        conflict(&project_overflow, &overflow_request),
        Revision3VoiceTakeStatusEditConflictV1::ProjectRevisionOverflow
    );

    let mut take_overflow = project;
    take_overflow.entities.get_mut(&id(4)).unwrap().revision = u64::MAX;
    let take_overflow_request = request(
        &take_overflow,
        id(4),
        VoiceTakeStatus::Recorded,
        VoiceTakeStatus::Reviewed,
    );
    assert_eq!(
        conflict(&take_overflow, &take_overflow_request),
        Revision3VoiceTakeStatusEditConflictV1::VoiceTakeRevisionOverflow { take: id(4) }
    );
}

#[test]
fn request_is_bounded_duplicate_safe_canonical_and_field_ordered() {
    let project = basis();
    let request = request(
        &project,
        id(4),
        VoiceTakeStatus::Recorded,
        VoiceTakeStatus::Reviewed,
    );
    let canonical = request.to_canonical_json().unwrap();
    assert_eq!(
        Revision3VoiceTakeStatusEditRequestV1::from_json(&canonical).unwrap(),
        request
    );

    let ordered_fields = [
        "\"expected_head\"",
        "\"expected_project_id\"",
        "\"expected_revision\"",
        "\"expected_target\"",
        "\"line_id\"",
        "\"localization_id\"",
        "\"expected_loc_id\"",
        "\"locale\"",
        "\"slot_id\"",
        "\"expected_slot_revision\"",
        "\"take_id\"",
        "\"expected_take_revision\"",
        "\"expected_status\"",
        "\"desired_status\"",
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
        Revision3VoiceTakeStatusEditRequestV1::from_json(&format!(" {canonical}")),
        Err(Revision3VoiceTakeStatusEditRequestJsonErrorV1::NonCanonicalJson)
    ));
    let duplicate = canonical.replacen(
        "\"expected_revision\":11",
        "\"expected_revision\":11,\"expected_revision\":11",
        1,
    );
    assert!(matches!(
        Revision3VoiceTakeStatusEditRequestV1::from_json(&duplicate),
        Err(Revision3VoiceTakeStatusEditRequestJsonErrorV1::InvalidJson(
            _
        ))
    ));
    let unknown = canonical.replacen('{', "{\"unknown\":true,", 1);
    assert!(matches!(
        Revision3VoiceTakeStatusEditRequestV1::from_json(&unknown),
        Err(Revision3VoiceTakeStatusEditRequestJsonErrorV1::InvalidJson(
            _
        ))
    ));
    let desired_field = canonical.rfind(",\"desired_status\":").unwrap();
    let missing = format!("{}}}", &canonical[..desired_field]);
    assert!(matches!(
        Revision3VoiceTakeStatusEditRequestV1::from_json(&missing),
        Err(Revision3VoiceTakeStatusEditRequestJsonErrorV1::InvalidJson(
            _
        ))
    ));
    let wrong_type = canonical.replacen(
        "\"expected_take_revision\":6",
        "\"expected_take_revision\":\"6\"",
        1,
    );
    assert!(matches!(
        Revision3VoiceTakeStatusEditRequestV1::from_json(&wrong_type),
        Err(Revision3VoiceTakeStatusEditRequestJsonErrorV1::InvalidJson(
            _
        ))
    ));

    let oversized_json = "x".repeat(MAX_REVISION3_VOICE_TAKE_STATUS_EDIT_REQUEST_JSON_BYTES_V1 + 1);
    assert!(matches!(
        Revision3VoiceTakeStatusEditRequestV1::from_json(&oversized_json),
        Err(Revision3VoiceTakeStatusEditRequestJsonErrorV1::InputTooLarge { .. })
    ));
    let mut oversized_request = request;
    oversized_request.expected_loc_id =
        "X".repeat(MAX_REVISION3_VOICE_TAKE_STATUS_EDIT_REQUEST_JSON_BYTES_V1);
    assert!(matches!(
        oversized_request.to_canonical_json(),
        Err(Revision3VoiceTakeStatusEditRequestJsonErrorV1::InputTooLarge { .. })
    ));
}

#[test]
fn candidate_too_large_is_distinct_from_an_oversized_invalid_basis() {
    let mut project = basis();
    take_mut(&mut project, id(4)).status = VoiceTakeStatus::Draft;
    project.meta.author.clear();
    let unpadded = project.to_canonical_json().unwrap();
    project.meta.author = "x".repeat(MAX_PROJECT_JSON_BYTES - unpadded.len());
    let canonical_basis = project.to_canonical_json().unwrap();
    assert_eq!(canonical_basis.len(), MAX_PROJECT_JSON_BYTES);
    let request = request(
        &project,
        id(4),
        VoiceTakeStatus::Draft,
        VoiceTakeStatus::Approved,
    );
    let request_json = request.to_canonical_json().unwrap();

    let result = apply_revision3_voice_take_status_edit_transaction_v1(
        &basis_head(),
        &canonical_basis,
        &request_json,
    )
    .unwrap();
    let Revision3VoiceTakeStatusEditEvaluationV1::Rejected(rejection) = result else {
        panic!("status edit must not emit an oversized candidate")
    };
    assert!(matches!(
        rejection.conflict,
        Revision3VoiceTakeStatusEditConflictV1::CandidateTooLarge {
            actual,
            limit: MAX_PROJECT_JSON_BYTES,
        } if actual > MAX_PROJECT_JSON_BYTES
    ));

    let mut oversized_project = project;
    oversized_project.meta.author.push('x');
    let oversized_basis = serde_json::to_string(&oversized_project).unwrap();
    assert_eq!(oversized_basis.len(), MAX_PROJECT_JSON_BYTES + 1);
    assert!(matches!(
        apply_revision3_voice_take_status_edit_transaction_v1(
            &basis_head(),
            &oversized_basis,
            &request_json,
        ),
        Err(Revision3VoiceTakeStatusEditErrorV1::InvalidProject(
            ProjectRevision3JsonError::InputTooLarge {
                actual,
                limit: MAX_PROJECT_JSON_BYTES,
            }
        )) if actual == MAX_PROJECT_JSON_BYTES + 1
    ));
}
