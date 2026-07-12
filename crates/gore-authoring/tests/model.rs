use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::{
    ArchiveSeal, AssetMeta, AssetRef, AssetStoreIndex, ContentSeal, DiagnosticCode,
    DiagnosticSeverity, DialogLine, Entity, EntityId, EntityKind, EntityPayload, FixedHexError,
    FormatV2, GameGenerationAnchor, LocaleCode, LocalizationEntry, OggCodec, OggMetadata,
    OriginRef, ProjectId, ProjectJsonError, ProjectMeta, ProjectV2, SchemaRevisionV1, Sha256Digest,
    TypedRef, ValidationProfile, VoiceMemberProof, VoiceOperation, VoiceSlot, VoiceTake,
    VoiceTakeStatus, VoiceTarget, VoiceTargetResolution, MAX_PROJECT_JSON_BYTES,
};

fn entity_id(value: u128) -> EntityId {
    format!("{value:032x}").parse().unwrap()
}

fn project_id(value: u128) -> ProjectId {
    format!("{value:032x}").parse().unwrap()
}

fn authored_ref(id: EntityId, expected_kind: EntityKind) -> TypedRef {
    TypedRef::new(project_id(100), id, expected_kind)
}

fn digest(byte: &str) -> Sha256Digest {
    byte.repeat(32).parse().unwrap()
}

fn locale(value: &str) -> LocaleCode {
    value.parse().unwrap()
}

fn content_seal(byte: &str, byte_len: u64) -> ContentSeal {
    ContentSeal {
        byte_len,
        sha256: digest(byte),
    }
}

fn generation(byte: &str) -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: content_seal(byte, 1_000_000),
    }
}

fn new_origin(runtime_id: &str) -> OriginRef {
    OriginRef::New {
        authored_runtime_id: runtime_id.into(),
    }
}

fn imported_origin(byte: &str, byte_len: u64) -> OriginRef {
    OriginRef::Imported {
        importer: "voice_ogg_import_v1".into(),
        source_seal: content_seal(byte, byte_len),
        external_identity: None,
    }
}

fn localization_entity(id: EntityId, target: &GameGenerationAnchor) -> Entity {
    Entity {
        id,
        display_name: "Greeting text".into(),
        origin: OriginRef::Vanilla {
            generation: target.clone(),
            catalog_layer: "localization".into(),
            canonical_selector: "info_viper_greeting".into(),
            source_seal: content_seal("44", 8_192),
        },
        revision: 0,
        payload: EntityPayload::LocalizationEntry(LocalizationEntry {
            loc_id: "info_viper_greeting".into(),
            texts: BTreeMap::from([
                (locale("de"), "Hallo".into()),
                (locale("en"), "Hello".into()),
            ]),
        }),
    }
}

fn voice_take_entity(
    id: EntityId,
    language: &str,
    asset_byte: &str,
    byte_len: u64,
    status: VoiceTakeStatus,
) -> Entity {
    Entity {
        id,
        display_name: format!("{language} greeting take"),
        origin: imported_origin(asset_byte, byte_len),
        revision: 0,
        payload: EntityPayload::VoiceTake(VoiceTake {
            locale: locale(language),
            asset: AssetRef {
                sha256: digest(asset_byte),
                byte_len,
                logical_name: format!("greeting-{language}-{id}.ogg"),
            },
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

fn replace_target(archive: &str, member: &str) -> VoiceTarget {
    VoiceTarget {
        archive: archive.into(),
        member: member.into(),
        operation: VoiceOperation::Replace,
        archive_seal: ArchiveSeal {
            byte_len: 4_096,
            sha256: digest("22"),
        },
        member_proof: VoiceMemberProof::Present {
            uncompressed_size: 128,
            crc32: 0x1234_5678,
        },
    }
}

fn voice_slot_entity(
    id: EntityId,
    language: &str,
    target: VoiceTarget,
    candidates: Vec<EntityId>,
    selected: Option<EntityId>,
) -> Entity {
    Entity {
        id,
        display_name: format!("{language} greeting slot"),
        origin: new_origin(&format!("voice-slot:{id}")),
        revision: 0,
        payload: EntityPayload::VoiceSlot(VoiceSlot {
            locale: locale(language),
            target_resolution: VoiceTargetResolution::Resolved { target },
            candidates: candidates
                .into_iter()
                .map(|id| authored_ref(id, EntityKind::VoiceTake))
                .collect(),
            selected: selected.map(|id| authored_ref(id, EntityKind::VoiceTake)),
        }),
    }
}

fn line_entity(id: EntityId, localization: EntityId, slot: EntityId) -> Entity {
    Entity {
        id,
        display_name: "Viper greeting".into(),
        origin: new_origin("dialog-line:info_viper_greeting"),
        revision: 0,
        payload: EntityPayload::DialogLine(DialogLine {
            localization: authored_ref(localization, EntityKind::LocalizationEntry),
            speaker_hint: Some("viper".into()),
            voice_slots: BTreeMap::from([(
                locale("de"),
                authored_ref(slot, EntityKind::VoiceSlot),
            )]),
        }),
    }
}

fn valid_project() -> ProjectV2 {
    let target = generation("33");
    let localization = localization_entity(entity_id(1), &target);
    let approved = voice_take_entity(entity_id(2), "de", "11", 128, VoiceTakeStatus::Approved);
    let alternate = voice_take_entity(entity_id(5), "de", "12", 256, VoiceTakeStatus::Reviewed);
    let slot = voice_slot_entity(
        entity_id(4),
        "de",
        replace_target("german_new.zip", "NPC/Viper/greeting.ogg"),
        vec![approved.id, alternate.id],
        Some(approved.id),
    );
    let line = line_entity(entity_id(3), localization.id, slot.id);

    ProjectV2 {
        format: FormatV2,
        schema_revision: SchemaRevisionV1,
        project_id: project_id(100),
        revision: 7,
        meta: ProjectMeta {
            name: "VoiceFixture".into(),
            version: "1.0".into(),
            author: "tester".into(),
        },
        target,
        authoring_locales: BTreeSet::from([locale("de"), locale("en")]),
        // Deliberately insert in reverse order; BTreeMap defines canonical wire order.
        entities: BTreeMap::from([
            (alternate.id, alternate),
            (slot.id, slot),
            (line.id, line),
            (approved.id, approved),
            (localization.id, localization),
        ]),
        asset_store: AssetStoreIndex {
            assets: BTreeMap::from([
                (
                    digest("11"),
                    AssetMeta {
                        byte_len: 128,
                        media_type: "audio/ogg".into(),
                    },
                ),
                (
                    digest("12"),
                    AssetMeta {
                        byte_len: 256,
                        media_type: "audio/ogg".into(),
                    },
                ),
            ]),
        },
    }
}

fn payload_mut(project: &mut ProjectV2, id: u128) -> &mut EntityPayload {
    &mut project.entities.get_mut(&entity_id(id)).unwrap().payload
}

fn resolved_target_mut(slot: &mut VoiceSlot) -> &mut VoiceTarget {
    let VoiceTargetResolution::Resolved { target } = &mut slot.target_resolution else {
        panic!("fixture target is not resolved");
    };
    target
}

fn codes(project: &ProjectV2) -> Vec<DiagnosticCode> {
    project
        .validate()
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn canonical_json_round_trip_is_byte_identical() {
    let project = valid_project();
    assert!(project.validate().is_empty());

    let first = project.to_canonical_json().unwrap();
    let reopened = ProjectV2::from_json(&first).unwrap();
    let second = reopened.to_canonical_json().unwrap();

    assert_eq!(first, second);
    assert_eq!(project, reopened);
    assert!(first.contains("\"format\":2"));
    assert!(first.contains("\"schema_revision\":1"));
    assert!(first.contains("00000000000000000000000000000001"));
}

#[test]
fn project_json_size_limit_accepts_boundary_and_rejects_one_byte_over() {
    let mut json = valid_project().to_canonical_json().unwrap();
    assert!(json.len() < MAX_PROJECT_JSON_BYTES);
    json.push_str(&" ".repeat(MAX_PROJECT_JSON_BYTES - json.len()));

    assert!(ProjectV2::from_json(&json).is_ok());

    json.push(' ');
    assert!(matches!(
        ProjectV2::from_json(&json),
        Err(ProjectJsonError::InputTooLarge { actual, limit })
            if actual == MAX_PROJECT_JSON_BYTES + 1 && limit == MAX_PROJECT_JSON_BYTES
    ));
}

#[test]
fn strict_json_rejects_unknown_fields_wrong_markers_and_key_id_mismatch() {
    let project = valid_project();
    let canonical = project.to_canonical_json().unwrap();
    let unknown_root = canonical.replacen("\"revision\":7", "\"revision\":7,\"mystery\":true", 1);
    assert!(ProjectV2::from_json(&unknown_root).is_err());

    let unknown_payload = canonical.replacen(
        "\"loc_id\":\"info_viper_greeting\"",
        "\"loc_id\":\"info_viper_greeting\",\"mystery\":true",
        1,
    );
    assert!(ProjectV2::from_json(&unknown_payload).is_err());

    let wrong_format = canonical.replacen("\"format\":2", "\"format\":1", 1);
    assert!(ProjectV2::from_json(&wrong_format).is_err());
    let wrong_schema = canonical.replacen("\"schema_revision\":1", "\"schema_revision\":2", 1);
    assert!(ProjectV2::from_json(&wrong_schema).is_err());

    let first_id = entity_id(1);
    let mismatched_key = canonical.replacen(
        &format!("\"{first_id}\":"),
        &format!("\"{}\":", entity_id(99)),
        1,
    );
    assert!(ProjectV2::from_json(&mismatched_key).is_err());
}

#[test]
fn duplicate_json_keys_maps_and_authoring_locales_are_rejected() {
    let project = valid_project();
    let canonical = project.to_canonical_json().unwrap();

    let duplicate_field = canonical.replacen("\"revision\":7", "\"revision\":7,\"revision\":8", 1);
    assert!(ProjectV2::from_json(&duplicate_field).is_err());

    let id = entity_id(1);
    let entity_json = serde_json::to_string(project.entities.get(&id).unwrap()).unwrap();
    let entry = format!("\"{id}\":{entity_json}");
    let duplicate_entity = canonical.replacen(&entry, &format!("{entry},{entry}"), 1);
    assert!(ProjectV2::from_json(&duplicate_entity).is_err());

    let duplicate_text =
        canonical.replacen("\"de\":\"Hallo\"", "\"de\":\"Hallo\",\"de\":\"Servus\"", 1);
    assert!(ProjectV2::from_json(&duplicate_text).is_err());

    let duplicate_locale = canonical.replacen(
        "\"authoring_locales\":[\"de\",\"en\"]",
        "\"authoring_locales\":[\"de\",\"de\",\"en\"]",
        1,
    );
    assert!(ProjectV2::from_json(&duplicate_locale).is_err());
}

#[test]
fn malformed_ids_and_hashes_fail_before_a_project_exists() {
    assert!(matches!(
        "1234".parse::<EntityId>(),
        Err(FixedHexError::InvalidLength {
            expected: 32,
            actual: 4
        })
    ));
    assert!(matches!(
        "0000000000000000000000000000000A".parse::<EntityId>(),
        Err(FixedHexError::InvalidCharacter { index: 31, .. })
    ));
    assert!(matches!(
        "gg".repeat(32).parse::<Sha256Digest>(),
        Err(FixedHexError::InvalidCharacter { index: 0, .. })
    ));

    let canonical = valid_project().to_canonical_json().unwrap();
    let uppercase_id = canonical.replacen(
        "00000000000000000000000000000001",
        "0000000000000000000000000000000A",
        1,
    );
    assert!(ProjectV2::from_json(&uppercase_id).is_err());
    let short_hash = canonical.replacen(&"11".repeat(32), "11", 1);
    assert!(ProjectV2::from_json(&short_hash).is_err());
}

#[test]
fn line_reference_validation_distinguishes_declared_missing_and_target_kind() {
    let mut project = valid_project();
    let missing_id = entity_id(999);
    let EntityPayload::DialogLine(line) = payload_mut(&mut project, 3) else {
        panic!("fixture line changed kind");
    };
    line.localization.id = entity_id(2);
    line.voice_slots.insert(
        locale("en"),
        authored_ref(missing_id, EntityKind::DialogLine),
    );

    let diagnostics = project.validate();
    let actual_codes = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect::<Vec<_>>();
    assert!(actual_codes.contains(&DiagnosticCode::ReferenceTargetKindMismatch));
    assert!(actual_codes.contains(&DiagnosticCode::ReferenceDeclaredKindMismatch));
    assert!(actual_codes.contains(&DiagnosticCode::MissingReference));
    assert!(diagnostics.iter().all(|diagnostic| diagnostic.blocks_build));
    assert_eq!(
        serde_json::to_string(&diagnostics).unwrap(),
        serde_json::to_string(&project.validate()).unwrap()
    );
}

#[test]
fn every_authored_reference_must_name_the_containing_project() {
    let mut project = valid_project();
    let foreign_project = project_id(200);

    let EntityPayload::DialogLine(line) = payload_mut(&mut project, 3) else {
        panic!("fixture line changed kind");
    };
    line.localization.project_id = foreign_project;
    line.voice_slots.get_mut(&locale("de")).unwrap().project_id = foreign_project;

    let EntityPayload::VoiceSlot(slot) = payload_mut(&mut project, 4) else {
        panic!("fixture slot changed kind");
    };
    slot.candidates[0].project_id = foreign_project;
    slot.selected.as_mut().unwrap().project_id = foreign_project;

    project.entities.get_mut(&entity_id(5)).unwrap().origin = OriginRef::Generated {
        generator_id: "voice-normalize".into(),
        generator_version: 1,
        owner: TypedRef::new(foreign_project, entity_id(2), EntityKind::VoiceTake),
    };

    let diagnostics = project.validate();
    assert_eq!(diagnostics.len(), 5);
    assert!(diagnostics
        .iter()
        .all(|diagnostic| diagnostic.code == DiagnosticCode::ReferenceProjectMismatch));
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.property_path.as_deref().unwrap())
            .collect::<Vec<_>>(),
        [
            "payload.data.localization",
            "payload.data.voice_slots.de",
            "payload.data.candidates.0",
            "payload.data.selected",
            "origin.owner",
        ]
    );
}

#[test]
fn voice_slot_supports_multiple_candidates_and_selected_take() {
    let project = valid_project();
    let EntityPayload::VoiceSlot(slot) = &project.entities[&entity_id(4)].payload else {
        panic!("fixture slot changed kind");
    };
    assert_eq!(slot.candidates.len(), 2);
    assert_eq!(slot.selected.as_ref().unwrap().id, entity_id(2));
    assert!(project.validate().is_empty());
}

#[test]
fn unresolved_zero_match_round_trips_and_blocks_both_profiles() {
    let mut project = valid_project();
    let EntityPayload::VoiceSlot(slot) = payload_mut(&mut project, 4) else {
        panic!("fixture slot changed kind");
    };
    slot.target_resolution = VoiceTargetResolution::Unresolved;

    let json = project.to_canonical_json().unwrap();
    let reopened = ProjectV2::from_json(&json).unwrap();
    assert_eq!(json, reopened.to_canonical_json().unwrap());
    for profile in [
        ValidationProfile::Production,
        ValidationProfile::Experimental,
    ] {
        let unresolved = reopened
            .validate_with_profile(profile)
            .into_iter()
            .filter(|diagnostic| diagnostic.code == DiagnosticCode::UnresolvedVoiceTarget)
            .collect::<Vec<_>>();
        assert_eq!(unresolved.len(), 1);
        assert_eq!(unresolved[0].severity, DiagnosticSeverity::Error);
        assert!(unresolved[0].blocks_build);
    }
}

#[test]
fn ambiguous_multiple_matches_round_trip_exact_target_facts_and_block() {
    let mut project = valid_project();
    let candidates = vec![
        replace_target("german_new.zip", "NPC/Viper/greeting.ogg"),
        replace_target("german.zip", "Legacy/Viper/greeting.ogg"),
    ];
    let EntityPayload::VoiceSlot(slot) = payload_mut(&mut project, 4) else {
        panic!("fixture slot changed kind");
    };
    slot.target_resolution = VoiceTargetResolution::Ambiguous {
        candidates: candidates.clone(),
    };

    let json = project.to_canonical_json().unwrap();
    let reopened = ProjectV2::from_json(&json).unwrap();
    assert_eq!(json, reopened.to_canonical_json().unwrap());
    let EntityPayload::VoiceSlot(reopened_slot) = &reopened.entities[&entity_id(4)].payload else {
        panic!("reopened slot changed kind");
    };
    assert_eq!(
        reopened_slot.target_resolution,
        VoiceTargetResolution::Ambiguous { candidates }
    );
    for profile in [
        ValidationProfile::Production,
        ValidationProfile::Experimental,
    ] {
        let ambiguous = reopened
            .validate_with_profile(profile)
            .into_iter()
            .filter(|diagnostic| diagnostic.code == DiagnosticCode::AmbiguousVoiceTarget)
            .collect::<Vec<_>>();
        assert_eq!(ambiguous.len(), 1);
        assert_eq!(ambiguous[0].severity, DiagnosticSeverity::Error);
        assert!(ambiguous[0].blocks_build);
    }
}

#[test]
fn ambiguous_target_resolution_requires_two_or_more_candidates() {
    for candidate_count in 0..=2 {
        let mut project = valid_project();
        let candidates = vec![
            replace_target("german_new.zip", "NPC/Viper/greeting.ogg"),
            replace_target("german.zip", "Legacy/Viper/greeting.ogg"),
        ];
        let EntityPayload::VoiceSlot(slot) = payload_mut(&mut project, 4) else {
            panic!("fixture slot changed kind");
        };
        slot.target_resolution = VoiceTargetResolution::Ambiguous {
            candidates: candidates.into_iter().take(candidate_count).collect(),
        };

        let diagnostics = project.validate();
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == DiagnosticCode::AmbiguousVoiceTarget)
                .count(),
            1
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic.code == DiagnosticCode::InvalidAmbiguousTargetCardinality
                })
                .count(),
            usize::from(candidate_count < 2)
        );
    }
}

#[test]
fn resolved_match_round_trips_without_resolution_diagnostic() {
    let project = valid_project();
    let json = project.to_canonical_json().unwrap();
    let reopened = ProjectV2::from_json(&json).unwrap();
    assert_eq!(json, reopened.to_canonical_json().unwrap());
    assert!(reopened.validate().iter().all(|diagnostic| !matches!(
        diagnostic.code,
        DiagnosticCode::UnresolvedVoiceTarget | DiagnosticCode::AmbiguousVoiceTarget
    )));
}

#[test]
fn ambiguous_candidates_are_structurally_validated_and_folded_unique() {
    let mut project = valid_project();
    let mut invalid = replace_target("../bad.zip", "NPC/Viper/greeting.ogg");
    invalid.archive_seal.byte_len = 0;
    let duplicate = replace_target("GERMAN_NEW.ZIP", "npc/viper/GREETING.ogg");
    let EntityPayload::VoiceSlot(slot) = payload_mut(&mut project, 4) else {
        panic!("fixture slot changed kind");
    };
    slot.target_resolution = VoiceTargetResolution::Ambiguous {
        candidates: vec![
            replace_target("german_new.zip", "NPC/Viper/greeting.ogg"),
            duplicate,
            invalid,
        ],
    };

    let diagnostics = project.validate();
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::DuplicateVoiceTargetCandidate));
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::InvalidVoiceTarget));
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::InvalidArchiveSeal));
    assert!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagnosticCode::DuplicateVoiceTarget)
            .count()
            == 0
    );
}

#[test]
fn duplicate_candidate_unlink_and_invalid_selection_have_distinct_diagnostics() {
    let mut duplicate = valid_project();
    let EntityPayload::VoiceSlot(slot) = payload_mut(&mut duplicate, 4) else {
        panic!("fixture slot changed kind");
    };
    slot.candidates.push(slot.candidates[0].clone());
    assert!(codes(&duplicate).contains(&DiagnosticCode::DuplicateVoiceCandidate));

    let mut unlinked = valid_project();
    let EntityPayload::VoiceSlot(slot) = payload_mut(&mut unlinked, 4) else {
        panic!("fixture slot changed kind");
    };
    slot.selected = None;
    assert_eq!(
        codes(&unlinked)
            .into_iter()
            .filter(|code| *code == DiagnosticCode::MissingSelectedVoiceTake)
            .count(),
        1
    );

    let mut not_candidate = valid_project();
    let EntityPayload::VoiceSlot(slot) = payload_mut(&mut not_candidate, 4) else {
        panic!("fixture slot changed kind");
    };
    slot.candidates
        .retain(|candidate| candidate.id != entity_id(2));
    assert!(codes(&not_candidate).contains(&DiagnosticCode::SelectedVoiceTakeNotCandidate));

    let mut not_approved = valid_project();
    let EntityPayload::VoiceTake(take) = payload_mut(&mut not_approved, 2) else {
        panic!("fixture take changed kind");
    };
    take.status = VoiceTakeStatus::Reviewed;
    assert!(codes(&not_approved).contains(&DiagnosticCode::SelectedVoiceTakeNotApproved));
}

#[test]
fn one_take_can_be_reused_explicitly_by_multiple_slots() {
    let mut project = valid_project();
    let reused_slot = voice_slot_entity(
        entity_id(6),
        "de",
        replace_target("german_new.zip", "NPC/Viper/farewell.ogg"),
        vec![entity_id(2)],
        Some(entity_id(2)),
    );
    project.entities.insert(reused_slot.id, reused_slot);
    assert!(project.validate().is_empty());
}

#[test]
fn locale_and_folded_target_collisions_are_deterministic() {
    let mut project = valid_project();
    let colliding_slot = voice_slot_entity(
        entity_id(6),
        "de",
        replace_target("GERMAN_NEW.ZIP", "npc/viper/GREETING.ogg"),
        vec![entity_id(2)],
        Some(entity_id(2)),
    );
    project.entities.insert(colliding_slot.id, colliding_slot);

    let EntityPayload::VoiceSlot(slot) = payload_mut(&mut project, 4) else {
        panic!("fixture slot changed kind");
    };
    slot.locale = locale("en");

    let diagnostics = project.validate();
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagnosticCode::DuplicateVoiceTarget)
            .count(),
        1
    );
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::LocaleSlotMismatch));
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::SlotTakeLocaleMismatch));
    let duplicate = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagnosticCode::DuplicateVoiceTarget)
        .unwrap();
    assert_eq!(duplicate.entity, Some(entity_id(6)));
    assert_eq!(duplicate.related_entities, [entity_id(4)]);
}

#[test]
fn asset_store_requires_present_size_matching_assets() {
    let mut missing = valid_project();
    missing.asset_store.assets.remove(&digest("11"));
    let diagnostics = missing.validate();
    let missing_asset = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagnosticCode::MissingAsset)
        .unwrap();
    assert_eq!(missing_asset.entity, Some(entity_id(2)));

    let mut mismatched = valid_project();
    mismatched
        .asset_store
        .assets
        .get_mut(&digest("11"))
        .unwrap()
        .byte_len = 129;
    assert!(codes(&mismatched).contains(&DiagnosticCode::AssetSizeMismatch));
}

#[test]
fn voice_take_asset_requires_canonical_ogg_media_type() {
    let mut project = valid_project();
    project
        .asset_store
        .assets
        .get_mut(&digest("11"))
        .unwrap()
        .media_type = "Audio/Ogg".into();

    let diagnostics = project.validate();
    let mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == DiagnosticCode::AssetMediaTypeMismatch)
        .collect::<Vec<_>>();
    assert_eq!(mismatches.len(), 1);
    assert_eq!(mismatches[0].entity, Some(entity_id(2)));
    assert_eq!(
        mismatches[0].property_path.as_deref(),
        Some("payload.data.asset.sha256")
    );
}

#[test]
fn vanilla_origins_are_generation_qualified_and_typed_refs_remain_authored_only() {
    let mut project = valid_project();
    project.entities.get_mut(&entity_id(1)).unwrap().origin = OriginRef::Vanilla {
        generation: generation("99"),
        catalog_layer: "localization".into(),
        canonical_selector: "info_viper_greeting".into(),
        source_seal: content_seal("44", 8_192),
    };
    assert!(codes(&project).contains(&DiagnosticCode::OriginGenerationMismatch));

    let mut generated = valid_project();
    generated.entities.get_mut(&entity_id(5)).unwrap().origin = OriginRef::Generated {
        generator_id: "voice-normalize".into(),
        generator_version: 1,
        owner: authored_ref(entity_id(999), EntityKind::VoiceTake),
    };
    assert!(codes(&generated).contains(&DiagnosticCode::MissingReference));
}

#[test]
fn member_presence_proof_must_match_add_or_replace() {
    let mut replace_absent = valid_project();
    let EntityPayload::VoiceSlot(slot) = payload_mut(&mut replace_absent, 4) else {
        panic!("fixture slot changed kind");
    };
    resolved_target_mut(slot).member_proof = VoiceMemberProof::Absent;
    assert!(codes(&replace_absent).contains(&DiagnosticCode::MemberProofOperationMismatch));

    let mut add_present = valid_project();
    let EntityPayload::VoiceSlot(slot) = payload_mut(&mut add_present, 4) else {
        panic!("fixture slot changed kind");
    };
    resolved_target_mut(slot).operation = VoiceOperation::Add;
    assert!(codes(&add_present).contains(&DiagnosticCode::MemberProofOperationMismatch));
}

#[test]
fn unqualified_add_blocks_production_but_warns_in_experimental_profile() {
    let mut project = valid_project();
    let EntityPayload::VoiceSlot(slot) = payload_mut(&mut project, 4) else {
        panic!("fixture slot changed kind");
    };
    let target = resolved_target_mut(slot);
    target.operation = VoiceOperation::Add;
    target.member_proof = VoiceMemberProof::Absent;

    let production = project.validate_with_profile(ValidationProfile::Production);
    let production_add = production
        .iter()
        .find(|diagnostic| diagnostic.code == DiagnosticCode::UnqualifiedVoiceAdd)
        .unwrap();
    assert_eq!(production_add.severity, DiagnosticSeverity::Error);
    assert!(production_add.blocks_build);

    let experimental = project.validate_with_profile(ValidationProfile::Experimental);
    let experimental_add = experimental
        .iter()
        .find(|diagnostic| diagnostic.code == DiagnosticCode::UnqualifiedVoiceAdd)
        .unwrap();
    assert_eq!(experimental_add.severity, DiagnosticSeverity::Warning);
    assert!(!experimental_add.blocks_build);
    assert!(experimental
        .iter()
        .all(|diagnostic| diagnostic.code == DiagnosticCode::UnqualifiedVoiceAdd));

    let canonical = project.to_canonical_json().unwrap();
    let attempted_flag = canonical.replacen(
        "\"operation\":\"add\"",
        "\"operation\":\"add\",\"runtime_qualified\":true",
        1,
    );
    assert!(ProjectV2::from_json(&attempted_flag).is_err());
}

#[test]
fn missing_localization_values_are_reported_in_locale_order() {
    let mut project = valid_project();
    let EntityPayload::LocalizationEntry(localization) = payload_mut(&mut project, 1) else {
        panic!("fixture localization changed kind");
    };
    localization.texts.remove(&locale("de"));
    localization.texts.insert(locale("en"), "   ".into());

    let missing = project
        .validate()
        .into_iter()
        .filter(|diagnostic| diagnostic.code == DiagnosticCode::MissingLocalizationValue)
        .collect::<Vec<_>>();
    assert_eq!(missing.len(), 2);
    assert_eq!(
        missing
            .iter()
            .map(|diagnostic| diagnostic.property_path.as_deref().unwrap())
            .collect::<Vec<_>>(),
        ["payload.data.texts.de", "payload.data.texts.en"]
    );
}

#[test]
fn programmatic_key_id_mismatch_uses_map_key_as_diagnostic_owner() {
    let mut project = valid_project();
    let entity = project.entities.remove(&entity_id(5)).unwrap();
    let map_key = entity_id(50);
    project.entities.insert(map_key, entity);

    let diagnostics = project.validate();
    let mismatch = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagnosticCode::EntityKeyIdMismatch)
        .unwrap();
    assert_eq!(mismatch.entity, Some(map_key));
    assert!(diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.property_path.as_deref() == Some("payload.data.locale"))
        .all(|diagnostic| diagnostic.entity == Some(map_key)));

    let serialized = project.to_canonical_json().unwrap();
    assert!(ProjectV2::from_json(&serialized).is_err());
}
