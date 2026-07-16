use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::model_revision3::{
    DialogLine, Entity, EntityKind, EntityPayload, LocalizationEntry, OggCodec, OggMetadata,
    OriginRef, SchemaRevisionV3, TypedRef, VoiceMemberProof, VoiceOperation, VoiceSlot, VoiceTake,
    VoiceTakeStatus, VoiceTarget, VoiceTargetResolution,
};
use gore_authoring::{
    plan_revision3_voice_build_v1, ArchiveSeal, AssetMeta, AssetRef, AssetStoreIndex, ContentSeal,
    EntityId, FormatV2, GameGenerationAnchor, LocaleCode, ProjectId, ProjectMeta, ProjectRevision3,
    Revision3VoiceBuildBlockReasonV1, Revision3VoiceBuildPlanErrorV1,
    Revision3VoiceBuildPlanEvaluationV1, Sha256Digest,
    MAX_REVISION3_VOICE_BUILD_LINE_LABEL_BYTES_V1,
    MAX_REVISION3_VOICE_BUILD_SELECTED_PAYLOAD_BYTES_V1, MAX_REVISION3_VOICE_BUILD_SLOTS_V1,
    MAX_REVISION3_VOICE_TARGET_ARCHIVE_CONTENT_BYTES_V1,
    MAX_REVISION3_VOICE_TARGET_MEMBER_UNCOMPRESSED_BYTES_V1,
};

fn id(tag: u8) -> EntityId {
    EntityId::from_bytes([tag; 16])
}

fn wide_id(value: u32) -> EntityId {
    let mut bytes = [0u8; 16];
    bytes[0] = 0x80;
    bytes[12..].copy_from_slice(&value.to_be_bytes());
    EntityId::from_bytes(bytes)
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

fn locale() -> LocaleCode {
    "de".parse().unwrap()
}

fn origin(tag: u8) -> OriginRef {
    OriginRef::Imported {
        importer: "voice-build-tests".to_owned(),
        source_seal: seal(tag, 100),
        external_identity: None,
    }
}

fn existing_target(tag: u8, member: &str) -> VoiceTarget {
    VoiceTarget {
        archive: "german_new.zip".to_owned(),
        member: member.to_owned(),
        operation: VoiceOperation::Replace,
        archive_seal: ArchiveSeal {
            byte_len: 2048,
            sha256: digest(tag),
        },
        member_proof: VoiceMemberProof::Present {
            uncompressed_size: 8192,
            crc32: u32::from(tag),
        },
    }
}

fn project(resolution: VoiceTargetResolution, selected: bool) -> ProjectRevision3 {
    let project_id = ProjectId::from_bytes([0x10; 16]);
    let localization_id = id(1);
    let line_id = id(2);
    let slot_id = id(3);
    let take_id = id(4);
    let asset = AssetRef {
        sha256: digest(0x41),
        byte_len: 8192,
        logical_name: "GRD_263_ASGHAN_OPEN_INFO_06_02.ogg".to_owned(),
    };
    let locale = locale();
    ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id,
        revision: 9,
        meta: ProjectMeta {
            name: "AsghanVoice".to_owned(),
            version: "1.0.0".to_owned(),
            author: "tests".to_owned(),
        },
        target: GameGenerationAnchor {
            executable: seal(0x20, 171_698_176),
        },
        authoring_locales: BTreeSet::from([locale.clone()]),
        entities: BTreeMap::from([
            (
                localization_id,
                Entity {
                    id: localization_id,
                    display_name: "Asghan line text".to_owned(),
                    origin: origin(1),
                    revision: 1,
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
                    origin: origin(2),
                    revision: 1,
                    payload: EntityPayload::DialogLine(DialogLine {
                        localization: TypedRef::new(
                            project_id,
                            localization_id,
                            EntityKind::LocalizationEntry,
                        ),
                        speaker_hint: Some("Asghan".to_owned()),
                        voice_slots: BTreeMap::from([(
                            locale.clone(),
                            TypedRef::new(project_id, slot_id, EntityKind::VoiceSlot),
                        )]),
                    }),
                },
            ),
            (
                slot_id,
                Entity {
                    id: slot_id,
                    display_name: "Asghan DE".to_owned(),
                    origin: origin(3),
                    revision: 1,
                    payload: EntityPayload::VoiceSlot(VoiceSlot {
                        locale: locale.clone(),
                        target_resolution: resolution,
                        candidates: vec![TypedRef::new(project_id, take_id, EntityKind::VoiceTake)],
                        selected: selected
                            .then(|| TypedRef::new(project_id, take_id, EntityKind::VoiceTake)),
                    }),
                },
            ),
            (
                take_id,
                Entity {
                    id: take_id,
                    display_name: "Approved take".to_owned(),
                    origin: origin(4),
                    revision: 1,
                    payload: EntityPayload::VoiceTake(VoiceTake {
                        locale,
                        asset: asset.clone(),
                        ogg: OggMetadata {
                            codec: OggCodec::Vorbis,
                            channels: 1,
                            sample_rate: 48_000,
                            pages: 3,
                            logical_streams: 1,
                        },
                        status: VoiceTakeStatus::Approved,
                    }),
                },
            ),
        ]),
        asset_store: AssetStoreIndex {
            assets: BTreeMap::from([(
                asset.sha256,
                AssetMeta {
                    byte_len: asset.byte_len,
                    media_type: "audio/ogg".to_owned(),
                },
            )]),
        },
    }
}

fn resolved_target_mut(project: &mut ProjectRevision3) -> &mut VoiceTarget {
    let EntityPayload::VoiceSlot(slot) = &mut project.entities.get_mut(&id(3)).unwrap().payload
    else {
        unreachable!()
    };
    let VoiceTargetResolution::Resolved { target } = &mut slot.target_resolution else {
        unreachable!()
    };
    target
}

fn project_with_slot_count(count: usize) -> ProjectRevision3 {
    let mut project = project(VoiceTargetResolution::Unresolved, false);
    project.entities.clear();
    let asset = AssetRef {
        sha256: digest(0x41),
        byte_len: 8192,
        logical_name: "shared.ogg".to_owned(),
    };

    for index in 0..count {
        let base = u32::try_from(index).unwrap() * 4;
        let localization_id = wide_id(base + 1);
        let line_id = wide_id(base + 2);
        let slot_id = wide_id(base + 3);
        let take_id = wide_id(base + 4);
        let loc_id = format!("VOICE_LIMIT_{index}");
        let locale = locale();
        project.entities.extend([
            (
                localization_id,
                Entity {
                    id: localization_id,
                    display_name: format!("Line {index} text"),
                    origin: origin(1),
                    revision: 1,
                    payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                        loc_id: loc_id.clone(),
                        texts: BTreeMap::from([(locale.clone(), format!("Line {index}"))]),
                    }),
                },
            ),
            (
                line_id,
                Entity {
                    id: line_id,
                    display_name: format!("Line {index}"),
                    origin: origin(2),
                    revision: 1,
                    payload: EntityPayload::DialogLine(DialogLine {
                        localization: TypedRef::new(
                            project.project_id,
                            localization_id,
                            EntityKind::LocalizationEntry,
                        ),
                        speaker_hint: None,
                        voice_slots: BTreeMap::from([(
                            locale.clone(),
                            TypedRef::new(project.project_id, slot_id, EntityKind::VoiceSlot),
                        )]),
                    }),
                },
            ),
            (
                slot_id,
                Entity {
                    id: slot_id,
                    display_name: format!("Line {index} DE"),
                    origin: origin(3),
                    revision: 1,
                    payload: EntityPayload::VoiceSlot(VoiceSlot {
                        locale: locale.clone(),
                        target_resolution: VoiceTargetResolution::Resolved {
                            target: existing_target(0x51, &format!("Npc/Test/{loc_id}.ogg")),
                        },
                        candidates: vec![TypedRef::new(
                            project.project_id,
                            take_id,
                            EntityKind::VoiceTake,
                        )],
                        selected: Some(TypedRef::new(
                            project.project_id,
                            take_id,
                            EntityKind::VoiceTake,
                        )),
                    }),
                },
            ),
            (
                take_id,
                Entity {
                    id: take_id,
                    display_name: format!("Line {index} approved take"),
                    origin: origin(4),
                    revision: 1,
                    payload: EntityPayload::VoiceTake(VoiceTake {
                        locale,
                        asset: asset.clone(),
                        ogg: OggMetadata {
                            codec: OggCodec::Vorbis,
                            channels: 1,
                            sample_rate: 48_000,
                            pages: 3,
                            logical_streams: 1,
                        },
                        status: VoiceTakeStatus::Approved,
                    }),
                },
            ),
        ]);
    }
    project
}

fn two_slots_reusing_one_take(payload_bytes: u64) -> ProjectRevision3 {
    let mut project = project_with_slot_count(2);
    let shared_take_id = wide_id(4);
    let second_slot_id = wide_id(7);
    let second_take_id = wide_id(8);
    let shared_ref = TypedRef::new(project.project_id, shared_take_id, EntityKind::VoiceTake);
    let EntityPayload::VoiceSlot(second_slot) =
        &mut project.entities.get_mut(&second_slot_id).unwrap().payload
    else {
        unreachable!()
    };
    second_slot.candidates = vec![shared_ref.clone()];
    second_slot.selected = Some(shared_ref);
    project.entities.remove(&second_take_id);

    let EntityPayload::VoiceTake(shared_take) =
        &mut project.entities.get_mut(&shared_take_id).unwrap().payload
    else {
        unreachable!()
    };
    shared_take.asset.byte_len = payload_bytes;
    project
        .asset_store
        .assets
        .get_mut(&shared_take.asset.sha256)
        .unwrap()
        .byte_len = payload_bytes;
    project
}

#[test]
fn resolved_existing_member_and_approved_selection_lower_exactly() {
    let target = existing_target(0x51, "Npc/Asghan/GRD_263_ASGHAN_OPEN_INFO_06_02.ogg");
    let project = project(
        VoiceTargetResolution::Resolved {
            target: target.clone(),
        },
        true,
    );
    let Revision3VoiceBuildPlanEvaluationV1::Ready { plan } =
        plan_revision3_voice_build_v1(&project).unwrap()
    else {
        panic!("expected ready Voice plan")
    };
    assert_eq!(plan.schema_revision, 1);
    assert_eq!(plan.project_id, project.project_id);
    assert_eq!(plan.project_revision, 9);
    assert_eq!(plan.edits.len(), 1);
    assert_eq!(plan.edits[0].slot_id, id(3));
    assert_eq!(plan.edits[0].take_id, id(4));
    assert_eq!(plan.edits[0].target, target);
    assert_eq!(plan.edits[0].asset.sha256, digest(0x41));
}

#[test]
fn unresolved_target_and_missing_selection_are_both_explicit_blockers() {
    let project = project(VoiceTargetResolution::Unresolved, false);
    let Revision3VoiceBuildPlanEvaluationV1::Blocked { report } =
        plan_revision3_voice_build_v1(&project).unwrap()
    else {
        panic!("expected blocked Voice plan")
    };
    assert_eq!(report.total_slots, 1);
    assert_eq!(report.ready_slots, 0);
    for blocker in &report.blockers {
        assert_eq!(blocker.slot_id, Some(id(3)));
        assert_eq!(blocker.line_id, Some(id(2)));
        assert_eq!(blocker.line_label.as_deref(), Some("Asghan greeting"));
        assert_eq!(
            blocker.loc_id.as_deref(),
            Some("GRD_263_ASGHAN_OPEN_INFO_06_02")
        );
        assert_eq!(blocker.locale.as_ref(), Some(&locale()));
    }
    assert_eq!(
        report
            .blockers
            .iter()
            .map(|blocker| blocker.reason)
            .collect::<Vec<_>>(),
        vec![
            Revision3VoiceBuildBlockReasonV1::UnresolvedTarget,
            Revision3VoiceBuildBlockReasonV1::MissingSelectedTake,
        ]
    );
}

#[test]
fn selected_non_approved_take_is_valid_authoring_state_and_explicit_build_blocker() {
    let mut project = project(
        VoiceTargetResolution::Resolved {
            target: existing_target(0x51, "Npc/Asghan/line.ogg"),
        },
        true,
    );
    let EntityPayload::VoiceTake(take) = &mut project.entities.get_mut(&id(4)).unwrap().payload
    else {
        unreachable!()
    };
    take.status = VoiceTakeStatus::Reviewed;

    project.validate_closed_model().unwrap();
    let Revision3VoiceBuildPlanEvaluationV1::Blocked { report } =
        plan_revision3_voice_build_v1(&project).unwrap()
    else {
        panic!("selected reviewed take must block rather than invalidate authoring state")
    };
    assert_eq!(report.ready_slots, 0);
    assert_eq!(report.blockers.len(), 1);
    assert_eq!(
        report.blockers[0].reason,
        Revision3VoiceBuildBlockReasonV1::SelectedTakeNotApproved
    );
}

#[test]
fn ambiguous_target_never_selects_a_member_implicitly() {
    let project = project(
        VoiceTargetResolution::Ambiguous {
            candidates: vec![
                existing_target(0x51, "Npc/Asghan/line.ogg"),
                existing_target(0x51, "Dialog/Asghan/line.ogg"),
            ],
        },
        true,
    );
    let Revision3VoiceBuildPlanEvaluationV1::Blocked { report } =
        plan_revision3_voice_build_v1(&project).unwrap()
    else {
        panic!("expected blocked Voice plan")
    };
    assert_eq!(report.ready_slots, 0);
    assert_eq!(
        report.blockers[0].reason,
        Revision3VoiceBuildBlockReasonV1::AmbiguousTarget
    );
}

#[test]
fn project_without_voice_slots_is_not_a_silent_empty_build() {
    let mut project = project(VoiceTargetResolution::Unresolved, false);
    project.entities.remove(&id(3));
    project.entities.remove(&id(4));
    project.asset_store.assets.clear();
    let EntityPayload::DialogLine(line) = &mut project.entities.get_mut(&id(2)).unwrap().payload
    else {
        unreachable!()
    };
    line.voice_slots.clear();

    let Revision3VoiceBuildPlanEvaluationV1::Blocked { report } =
        plan_revision3_voice_build_v1(&project).unwrap()
    else {
        panic!("expected blocked Voice plan")
    };
    assert_eq!(report.total_slots, 0);
    assert_eq!(
        report.blockers[0].reason,
        Revision3VoiceBuildBlockReasonV1::NoVoiceSlots
    );
    assert_eq!(report.blockers[0].slot_id, None);
    assert_eq!(report.blockers[0].line_id, None);
    assert_eq!(report.blockers[0].line_label, None);
    assert_eq!(report.blockers[0].loc_id, None);
    assert_eq!(report.blockers[0].locale, None);
}

#[test]
fn managed_voice_build_slot_limit_accepts_1024_and_blocks_1025_globally() {
    let boundary = project_with_slot_count(MAX_REVISION3_VOICE_BUILD_SLOTS_V1);
    let Revision3VoiceBuildPlanEvaluationV1::Ready { plan } =
        plan_revision3_voice_build_v1(&boundary).unwrap()
    else {
        panic!("1024 managed Voice slots must remain buildable")
    };
    assert_eq!(plan.edits.len(), MAX_REVISION3_VOICE_BUILD_SLOTS_V1);

    let oversized = project_with_slot_count(MAX_REVISION3_VOICE_BUILD_SLOTS_V1 + 1);
    let Revision3VoiceBuildPlanEvaluationV1::Blocked { report } =
        plan_revision3_voice_build_v1(&oversized).unwrap()
    else {
        panic!("1025 managed Voice slots must return one bounded global blocker")
    };
    assert_eq!(
        report.total_slots,
        u64::try_from(MAX_REVISION3_VOICE_BUILD_SLOTS_V1 + 1).unwrap()
    );
    assert_eq!(report.ready_slots, 0);
    assert_eq!(report.blockers.len(), 1);
    let blocker = &report.blockers[0];
    assert_eq!(
        blocker.reason,
        Revision3VoiceBuildBlockReasonV1::VoiceSlotLimitExceeded
    );
    assert_eq!(blocker.slot_id, None);
    assert_eq!(blocker.line_id, None);
    assert_eq!(blocker.line_label, None);
    assert_eq!(blocker.loc_id, None);
    assert_eq!(blocker.locale, None);
}

#[test]
fn managed_voice_build_slot_limit_precedes_line_label_projection() {
    let mut oversized = project_with_slot_count(MAX_REVISION3_VOICE_BUILD_SLOTS_V1 + 1);
    oversized
        .entities
        .get_mut(&wide_id(2))
        .unwrap()
        .display_name = " Line 0".to_owned();

    let Revision3VoiceBuildPlanEvaluationV1::Blocked { report } =
        plan_revision3_voice_build_v1(&oversized).unwrap()
    else {
        panic!("the hard slot cap must precede presentation-fact projection")
    };
    assert_eq!(
        report.total_slots,
        u64::try_from(MAX_REVISION3_VOICE_BUILD_SLOTS_V1 + 1).unwrap()
    );
    assert_eq!(report.ready_slots, 0);
    assert_eq!(report.blockers.len(), 1);
    assert_eq!(
        report.blockers[0].reason,
        Revision3VoiceBuildBlockReasonV1::VoiceSlotLimitExceeded
    );
    assert_eq!(report.blockers[0].line_label, None);
}

#[test]
fn selected_payload_budget_counts_reused_take_per_slot_occurrence() {
    assert_eq!(
        MAX_REVISION3_VOICE_BUILD_SELECTED_PAYLOAD_BYTES_V1,
        256 * 1024 * 1024
    );
    let per_occurrence = MAX_REVISION3_VOICE_BUILD_SELECTED_PAYLOAD_BYTES_V1 / 2;
    let boundary = two_slots_reusing_one_take(per_occurrence);
    let Revision3VoiceBuildPlanEvaluationV1::Ready { plan } =
        plan_revision3_voice_build_v1(&boundary).unwrap()
    else {
        panic!("the exact 256 MiB aggregate boundary must remain buildable")
    };
    assert_eq!(plan.edits.len(), 2);
    assert!(plan.edits.iter().all(|edit| edit.take_id == wide_id(4)));

    let exceeded = two_slots_reusing_one_take(per_occurrence + 1);
    let Revision3VoiceBuildPlanEvaluationV1::Blocked { report } =
        plan_revision3_voice_build_v1(&exceeded).unwrap()
    else {
        panic!("a reused selected take must count once per planned slot occurrence")
    };
    assert_eq!(report.total_slots, 2);
    assert_eq!(report.ready_slots, 0);
    assert_eq!(report.blockers.len(), 1);
    let blocker = &report.blockers[0];
    assert_eq!(
        blocker.reason,
        Revision3VoiceBuildBlockReasonV1::VoicePayloadBudgetExceeded
    );
    assert_eq!(
        serde_json::to_value(blocker.reason).unwrap(),
        "voice_payload_budget_exceeded"
    );
    assert_eq!(blocker.slot_id, None);
    assert_eq!(blocker.line_id, None);
    assert_eq!(blocker.line_label, None);
    assert_eq!(blocker.loc_id, None);
    assert_eq!(blocker.locale, None);
}

#[test]
fn unicode_case_duplicate_targets_are_rejected_before_planning() {
    let mut project = project_with_slot_count(2);
    for (slot_id, archive) in [(wide_id(3), "Ä.zip"), (wide_id(7), "ä.zip")] {
        let EntityPayload::VoiceSlot(slot) =
            &mut project.entities.get_mut(&slot_id).unwrap().payload
        else {
            unreachable!()
        };
        let VoiceTargetResolution::Resolved { target } = &mut slot.target_resolution else {
            unreachable!()
        };
        target.archive = archive.to_owned();
        target.member = "Npc/Test/shared.ogg".to_owned();
    }

    assert!(matches!(
        plan_revision3_voice_build_v1(&project),
        Err(Revision3VoiceBuildPlanErrorV1::InvalidProject(message))
            if message.contains("duplicates the resolved archive/member target")
    ));
}

#[test]
fn blocker_line_label_contract_is_exact_canonical_and_utf8_bounded() {
    let mut project = project(VoiceTargetResolution::Unresolved, false);
    let exact_utf8_boundary = "é".repeat(MAX_REVISION3_VOICE_BUILD_LINE_LABEL_BYTES_V1 / 2);
    assert_eq!(
        exact_utf8_boundary.len(),
        MAX_REVISION3_VOICE_BUILD_LINE_LABEL_BYTES_V1
    );
    project.entities.get_mut(&id(2)).unwrap().display_name = exact_utf8_boundary.clone();
    let Revision3VoiceBuildPlanEvaluationV1::Blocked { report } =
        plan_revision3_voice_build_v1(&project).unwrap()
    else {
        panic!("unresolved target must block")
    };
    assert_eq!(
        report.blockers[0].line_label.as_deref(),
        Some(project.entities[&id(2)].display_name.as_str())
    );

    project.entities.get_mut(&id(2)).unwrap().display_name = "Asghan\u{00a0}greeting".to_owned();
    assert!(matches!(
        plan_revision3_voice_build_v1(&project),
        Ok(Revision3VoiceBuildPlanEvaluationV1::Blocked { .. })
    ));

    for invalid in [
        String::new(),
        " Asghan greeting".to_owned(),
        "Asghan greeting\u{00a0}".to_owned(),
        "\u{feff}Asghan greeting".to_owned(),
        "Asghan\u{009f}greeting".to_owned(),
        format!("{exact_utf8_boundary}x"),
    ] {
        project.entities.get_mut(&id(2)).unwrap().display_name = invalid;
        assert!(matches!(
            plan_revision3_voice_build_v1(&project),
            Err(Revision3VoiceBuildPlanErrorV1::InvalidProject(message))
                if message.contains("canonical build label")
        ));
    }
}

#[test]
fn approved_opus_take_remains_explicitly_build_blocked() {
    let mut project = project(
        VoiceTargetResolution::Resolved {
            target: existing_target(0x51, "Npc/Asghan/line.ogg"),
        },
        true,
    );
    let EntityPayload::VoiceTake(take) = &mut project.entities.get_mut(&id(4)).unwrap().payload
    else {
        unreachable!()
    };
    take.ogg.codec = OggCodec::Opus;

    let Revision3VoiceBuildPlanEvaluationV1::Blocked { report } =
        plan_revision3_voice_build_v1(&project).unwrap()
    else {
        panic!("approved Opus must not receive managed build authority")
    };
    assert_eq!(report.ready_slots, 0);
    assert_eq!(
        report.blockers[0].reason,
        Revision3VoiceBuildBlockReasonV1::SelectedTakeCodecUnqualified
    );
}

#[test]
fn unsafe_bundle_metadata_is_rejected_before_ready() {
    for name in ["", "../escape", "nested/mod", "nested\\mod", "bad\0name"] {
        let mut project = project(
            VoiceTargetResolution::Resolved {
                target: existing_target(0x51, "Npc/Asghan/line.ogg"),
            },
            true,
        );
        project.meta.name = name.to_owned();
        assert!(matches!(
            plan_revision3_voice_build_v1(&project),
            Err(Revision3VoiceBuildPlanErrorV1::InvalidProject(message))
                if message.contains("safe bundle name")
        ));
    }
}

#[test]
fn persisted_target_numeric_bounds_are_closed_before_planning() {
    let resolved = || {
        project(
            VoiceTargetResolution::Resolved {
                target: existing_target(0x51, "Npc/Asghan/line.ogg"),
            },
            true,
        )
    };

    let mut oversized_archive = resolved();
    resolved_target_mut(&mut oversized_archive)
        .archive_seal
        .byte_len = MAX_REVISION3_VOICE_TARGET_ARCHIVE_CONTENT_BYTES_V1 + 1;
    assert!(matches!(
        plan_revision3_voice_build_v1(&oversized_archive),
        Err(Revision3VoiceBuildPlanErrorV1::InvalidProject(message))
            if message.contains("archive seal byte length")
    ));

    let mut oversized_member = resolved();
    resolved_target_mut(&mut oversized_member).member_proof = VoiceMemberProof::Present {
        uncompressed_size: MAX_REVISION3_VOICE_TARGET_MEMBER_UNCOMPRESSED_BYTES_V1 + 1,
        crc32: 0x1234_5678,
    };
    assert!(matches!(
        plan_revision3_voice_build_v1(&oversized_member),
        Err(Revision3VoiceBuildPlanErrorV1::InvalidProject(message))
            if message.contains("uncompressed size")
    ));

    let mut boundary = resolved();
    let target = resolved_target_mut(&mut boundary);
    target.archive_seal.byte_len = MAX_REVISION3_VOICE_TARGET_ARCHIVE_CONTENT_BYTES_V1;
    target.member_proof = VoiceMemberProof::Present {
        uncompressed_size: MAX_REVISION3_VOICE_TARGET_MEMBER_UNCOMPRESSED_BYTES_V1,
        crc32: 0x1234_5678,
    };
    assert!(matches!(
        plan_revision3_voice_build_v1(&boundary).unwrap(),
        Revision3VoiceBuildPlanEvaluationV1::Ready { .. }
    ));
}
