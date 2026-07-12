use std::collections::{BTreeMap, BTreeSet};

use gore_authoring::model_revision2 as revision2;
use gore_authoring::{
    migrate_revision1_to_revision2, ArchiveSeal, AssetMeta, AssetRef, AssetStoreIndex, ContentSeal,
    DialogLine, Entity, EntityId, EntityKind, EntityPayload, FormatV2, GameGenerationAnchor,
    LocaleCode, LocalizationEntry, OggCodec, OggMetadata, OriginRef, ProjectDocument, ProjectId,
    ProjectMeta, ProjectV2, Revision1ToRevision2Error, Revision1ToRevision2Transformation,
    Revision1TypedRefPosition, SchemaRevisionV1, Sha256Digest, TypedRef, VoiceMemberProof,
    VoiceOperation, VoiceSlot, VoiceTake, VoiceTakeStatus, VoiceTarget, VoiceTargetResolution,
};

fn project_id(value: u128) -> ProjectId {
    format!("{value:032x}").parse().unwrap()
}

fn entity_id(value: u128) -> EntityId {
    format!("{value:032x}").parse().unwrap()
}

fn digest(byte: &str) -> Sha256Digest {
    byte.repeat(32).parse().unwrap()
}

fn locale(value: &str) -> LocaleCode {
    value.parse().unwrap()
}

fn authored_ref(project: ProjectId, id: u128, kind: EntityKind) -> TypedRef {
    TypedRef::new(project, entity_id(id), kind)
}

fn source_project() -> ProjectV2 {
    let source_project_id = project_id(10);
    let generation = GameGenerationAnchor {
        executable: ContentSeal {
            byte_len: 8_192,
            sha256: digest("aa"),
        },
    };
    let localization = Entity {
        id: entity_id(1),
        display_name: "Greeting text".into(),
        origin: OriginRef::Vanilla {
            generation: generation.clone(),
            catalog_layer: "localization".into(),
            canonical_selector: "info_greeting".into(),
            source_seal: ContentSeal {
                byte_len: 512,
                sha256: digest("bb"),
            },
        },
        revision: 2,
        payload: EntityPayload::LocalizationEntry(LocalizationEntry {
            loc_id: "info_greeting".into(),
            texts: BTreeMap::from([(locale("de"), "Hallo".into())]),
        }),
    };
    let take = Entity {
        id: entity_id(2),
        display_name: "Greeting take".into(),
        origin: OriginRef::Generated {
            generator_id: "voice-generator".into(),
            generator_version: 1,
            owner: authored_ref(source_project_id, 1, EntityKind::LocalizationEntry),
        },
        revision: 3,
        payload: EntityPayload::VoiceTake(VoiceTake {
            locale: locale("de"),
            asset: AssetRef {
                sha256: digest("cc"),
                byte_len: 128,
                logical_name: "greeting.ogg".into(),
            },
            ogg: OggMetadata {
                codec: OggCodec::Vorbis,
                channels: 1,
                sample_rate: 48_000,
                pages: 3,
                logical_streams: 1,
            },
            status: VoiceTakeStatus::Approved,
        }),
    };
    let slot = Entity {
        id: entity_id(3),
        display_name: "Greeting voice slot".into(),
        origin: OriginRef::New {
            authored_runtime_id: "voice-slot:greeting".into(),
        },
        revision: 1,
        payload: EntityPayload::VoiceSlot(VoiceSlot {
            locale: locale("de"),
            target_resolution: VoiceTargetResolution::Resolved {
                target: VoiceTarget {
                    archive: "german_new.zip".into(),
                    member: "NPC/Greeting.ogg".into(),
                    operation: VoiceOperation::Replace,
                    archive_seal: ArchiveSeal {
                        byte_len: 1_024,
                        sha256: digest("dd"),
                    },
                    member_proof: VoiceMemberProof::Present {
                        uncompressed_size: 128,
                        crc32: 0x1234_5678,
                    },
                },
            },
            candidates: vec![authored_ref(source_project_id, 2, EntityKind::VoiceTake)],
            selected: Some(authored_ref(source_project_id, 2, EntityKind::VoiceTake)),
        }),
    };
    let line = Entity {
        id: entity_id(4),
        display_name: "Greeting line".into(),
        origin: OriginRef::Imported {
            importer: "dialog-import".into(),
            source_seal: ContentSeal {
                byte_len: 256,
                sha256: digest("ee"),
            },
            external_identity: Some("INFO_GREETING".into()),
        },
        revision: 5,
        payload: EntityPayload::DialogLine(DialogLine {
            localization: authored_ref(source_project_id, 1, EntityKind::LocalizationEntry),
            speaker_hint: Some("viper".into()),
            voice_slots: BTreeMap::from([(
                locale("de"),
                authored_ref(source_project_id, 3, EntityKind::VoiceSlot),
            )]),
        }),
    };

    ProjectV2 {
        format: FormatV2,
        schema_revision: SchemaRevisionV1,
        project_id: source_project_id,
        revision: 11,
        meta: ProjectMeta {
            name: "Migrated project".into(),
            version: "2.3".into(),
            author: "tester".into(),
        },
        target: generation,
        authoring_locales: BTreeSet::from([locale("de")]),
        entities: BTreeMap::from([
            (line.id, line),
            (slot.id, slot),
            (take.id, take),
            (localization.id, localization),
        ]),
        asset_store: AssetStoreIndex {
            assets: BTreeMap::from([(
                digest("cc"),
                AssetMeta {
                    byte_len: 128,
                    media_type: "audio/ogg".into(),
                },
            )]),
        },
    }
}

fn revision2_project_ref_ids(project: &revision2::ProjectRevision2) -> Vec<ProjectId> {
    let mut ids = Vec::new();
    for entity in project.entities.values() {
        if let revision2::OriginRef::Generated { owner, .. } = &entity.origin {
            ids.push(owner.project_id);
        }
        match &entity.payload {
            revision2::EntityPayload::LocalizationEntry(_)
            | revision2::EntityPayload::VoiceTake(_) => {}
            revision2::EntityPayload::DialogLine(line) => {
                ids.push(line.localization.project_id);
                ids.extend(
                    line.voice_slots
                        .values()
                        .map(|reference| reference.project_id),
                );
            }
            revision2::EntityPayload::VoiceSlot(slot) => {
                ids.extend(slot.candidates.iter().map(|reference| reference.project_id));
                ids.extend(slot.selected.iter().map(|reference| reference.project_id));
            }
            revision2::EntityPayload::NpcDraft(draft) => {
                ids.push(draft.script_module.project_id);
            }
            revision2::EntityPayload::QuestDraft(draft) => {
                ids.push(draft.script_module.project_id);
            }
            revision2::EntityPayload::ScriptModule(module) => {
                ids.push(module.owner.project_id);
            }
        }
    }
    ids
}

fn fixture_ref_positions() -> Vec<Revision1TypedRefPosition> {
    vec![
        Revision1TypedRefPosition::EntityOriginGeneratedOwner {
            entity_key: entity_id(2),
        },
        Revision1TypedRefPosition::VoiceSlotCandidate {
            entity_key: entity_id(3),
            index: 0,
        },
        Revision1TypedRefPosition::VoiceSlotSelected {
            entity_key: entity_id(3),
        },
        Revision1TypedRefPosition::DialogLineLocalization {
            entity_key: entity_id(4),
        },
        Revision1TypedRefPosition::DialogLineVoiceSlot {
            entity_key: entity_id(4),
            locale: locale("de"),
        },
    ]
}

fn set_fixture_ref_project_id(
    source: &mut ProjectV2,
    position: &Revision1TypedRefPosition,
    project_id: ProjectId,
) {
    match position {
        Revision1TypedRefPosition::EntityOriginGeneratedOwner { entity_key } => {
            let OriginRef::Generated { owner, .. } =
                &mut source.entities.get_mut(entity_key).unwrap().origin
            else {
                panic!("fixture origin changed kind");
            };
            owner.project_id = project_id;
        }
        Revision1TypedRefPosition::DialogLineLocalization { entity_key } => {
            let EntityPayload::DialogLine(line) =
                &mut source.entities.get_mut(entity_key).unwrap().payload
            else {
                panic!("fixture line changed payload kind");
            };
            line.localization.project_id = project_id;
        }
        Revision1TypedRefPosition::DialogLineVoiceSlot { entity_key, locale } => {
            let EntityPayload::DialogLine(line) =
                &mut source.entities.get_mut(entity_key).unwrap().payload
            else {
                panic!("fixture line changed payload kind");
            };
            line.voice_slots.get_mut(locale).unwrap().project_id = project_id;
        }
        Revision1TypedRefPosition::VoiceSlotCandidate { entity_key, index } => {
            let EntityPayload::VoiceSlot(slot) =
                &mut source.entities.get_mut(entity_key).unwrap().payload
            else {
                panic!("fixture slot changed payload kind");
            };
            slot.candidates[usize::try_from(*index).unwrap()].project_id = project_id;
        }
        Revision1TypedRefPosition::VoiceSlotSelected { entity_key } => {
            let EntityPayload::VoiceSlot(slot) =
                &mut source.entities.get_mut(entity_key).unwrap().payload
            else {
                panic!("fixture slot changed payload kind");
            };
            slot.selected.as_mut().unwrap().project_id = project_id;
        }
    }
}

#[test]
fn migration_is_pure_deterministic_and_rewrites_every_internal_project_ref() {
    let source = source_project();
    let source_before = source.clone();
    let source_bytes_before = source.to_canonical_json().unwrap();
    let target_project_id = project_id(20);

    let first = migrate_revision1_to_revision2(&source, target_project_id).unwrap();
    let second = migrate_revision1_to_revision2(&source, target_project_id).unwrap();

    assert_eq!(source, source_before);
    assert_eq!(source.to_canonical_json().unwrap(), source_bytes_before);
    assert_eq!(first, second);
    assert_eq!(
        first.project.to_canonical_json().unwrap(),
        second.project.to_canonical_json().unwrap()
    );
    assert_eq!(
        serde_json::to_string(&first.report).unwrap(),
        serde_json::to_string(&second.report).unwrap()
    );
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );

    assert_eq!(first.project.project_id, target_project_id);
    assert_eq!(first.project.revision, source.revision);
    assert_eq!(first.project.meta, source.meta);
    assert_eq!(first.project.target, source.target);
    assert_eq!(first.project.asset_store, source.asset_store);
    assert_eq!(first.project.entities.len(), source.entities.len());
    assert_eq!(
        first.project.entities.keys().copied().collect::<Vec<_>>(),
        source.entities.keys().copied().collect::<Vec<_>>()
    );

    let rewritten_ids = revision2_project_ref_ids(&first.project);
    assert_eq!(rewritten_ids.len(), 5);
    assert!(rewritten_ids
        .iter()
        .all(|project_id| *project_id == target_project_id));
    assert!(!rewritten_ids.contains(&source.project_id));
    assert_eq!(first.report.migrated_entities, 4);
    assert_eq!(first.report.rewritten_internal_project_refs, 5);
    assert_eq!(
        first.report.transformations,
        [
            Revision1ToRevision2Transformation::ProjectIdentityReassigned {
                source_project_id: source.project_id,
                target_project_id,
            },
            Revision1ToRevision2Transformation::InternalProjectReferencesRewritten {
                source_project_id: source.project_id,
                target_project_id,
                count: 5,
            },
        ]
    );

    let canonical = first.project.to_canonical_json().unwrap();
    assert!(canonical.contains("\"schema_revision\":2"));
    assert!(matches!(
        ProjectDocument::from_json(&canonical).unwrap(),
        ProjectDocument::Revision2(_)
    ));
}

#[test]
fn migration_preserves_foreign_refs_and_counts_only_internal_rewrites() {
    let mut source = source_project();
    let foreign_project_id = project_id(99);
    let EntityPayload::DialogLine(line) =
        &mut source.entities.get_mut(&entity_id(4)).unwrap().payload
    else {
        panic!("fixture line changed payload kind");
    };
    line.localization.project_id = foreign_project_id;

    let migrated = migrate_revision1_to_revision2(&source, project_id(20)).unwrap();
    let revision2::EntityPayload::DialogLine(line) =
        &migrated.project.entities[&entity_id(4)].payload
    else {
        panic!("migrated line changed payload kind");
    };
    assert_eq!(line.localization.project_id, foreign_project_id);
    assert_eq!(migrated.report.rewritten_internal_project_refs, 4);
}

#[test]
fn every_typed_ref_position_rejects_a_foreign_id_that_would_become_internal() {
    let target_project_id = project_id(20);
    let positions = fixture_ref_positions();

    for position in &positions {
        let mut source = source_project();
        set_fixture_ref_project_id(&mut source, position, target_project_id);
        let source_before = source.clone();

        let error = migrate_revision1_to_revision2(&source, target_project_id).unwrap_err();
        assert_eq!(source, source_before);
        assert_eq!(
            error,
            Revision1ToRevision2Error::ForeignReferenceWouldBecomeInternal {
                target_project_id,
                conflicting_refs: 1,
                first_position: position.clone(),
            }
        );
        assert!(error.to_string().contains(&position.to_string()));
    }

    let mut source = source_project();
    for position in &positions {
        set_fixture_ref_project_id(&mut source, position, target_project_id);
    }
    assert_eq!(
        migrate_revision1_to_revision2(&source, target_project_id).unwrap_err(),
        Revision1ToRevision2Error::ForeignReferenceWouldBecomeInternal {
            target_project_id,
            conflicting_refs: u64::try_from(positions.len()).unwrap(),
            first_position: Revision1TypedRefPosition::EntityOriginGeneratedOwner {
                entity_key: entity_id(2),
            },
        }
    );
}

#[test]
fn migration_rejects_programmatic_entity_key_id_mismatch_before_output() {
    let mut source = source_project();
    let mismatched_key = entity_id(99);
    let entity = source.entities.remove(&entity_id(4)).unwrap();
    let embedded_id = entity.id;
    source.entities.insert(mismatched_key, entity);
    let source_before = source.clone();

    assert_eq!(
        migrate_revision1_to_revision2(&source, project_id(20)).unwrap_err(),
        Revision1ToRevision2Error::EntityKeyIdMismatch {
            key: mismatched_key,
            embedded_id,
        }
    );
    assert_eq!(source, source_before);
}

#[test]
fn migration_requires_an_explicitly_distinct_project_id() {
    let source = source_project();
    assert_eq!(
        migrate_revision1_to_revision2(&source, source.project_id).unwrap_err(),
        Revision1ToRevision2Error::ProjectIdNotChanged {
            project_id: source.project_id,
        }
    );
}

#[test]
fn generated_transformation_report_has_one_exact_closed_shape() {
    let source = source_project();
    let report = migrate_revision1_to_revision2(&source, project_id(20))
        .unwrap()
        .report;
    let canonical = serde_json::to_string(&report).unwrap();
    assert_eq!(
        canonical,
        format!(
            concat!(
                "{{\"format\":2,\"source_schema_revision\":1,\"target_schema_revision\":2,",
                "\"source_project_id\":\"{}\",\"target_project_id\":\"{}\",",
                "\"migrated_entities\":4,\"rewritten_internal_project_refs\":5,",
                "\"transformations\":[",
                "{{\"type\":\"project_identity_reassigned\",\"source_project_id\":\"{}\",",
                "\"target_project_id\":\"{}\"}},",
                "{{\"type\":\"internal_project_references_rewritten\",",
                "\"source_project_id\":\"{}\",\"target_project_id\":\"{}\",\"count\":5}}]}}"
            ),
            source.project_id,
            project_id(20),
            source.project_id,
            project_id(20),
            source.project_id,
            project_id(20),
        )
    );
}
