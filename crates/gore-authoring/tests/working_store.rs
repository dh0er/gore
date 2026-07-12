use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use gore_authoring::{
    ArchiveSeal, AssetMeta, AssetStoreIndex, AssetVerification, ContentSeal, DiagnosticCode,
    DiagnosticSeverity, DialogLine, Entity, EntityId, EntityKind, EntityPayload, FormatV2,
    GameGenerationAnchor, LocaleCode, LocalizationEntry, OggCodec, OriginRef, ProjectId,
    ProjectMeta, ProjectV2, SchemaRevisionV1, Sha256Digest, TypedRef, ValidationProfile,
    VoiceMemberProof, VoiceOperation, VoiceSlot, VoiceTake, VoiceTakeStatus, VoiceTarget,
    VoiceTargetResolution, WorkingHead, WorkingProjectStore, WorkingStoreError, WorkingStoreLimits,
};
use sha2::{Digest, Sha256};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);

impl TestRoot {
    fn new(label: &str) -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "gore-authoring-store-{label}-{}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn entity_id(value: u128) -> EntityId {
    format!("{value:032x}").parse().unwrap()
}

fn project_id(value: u128) -> ProjectId {
    format!("{value:032x}").parse().unwrap()
}

fn locale(value: &str) -> LocaleCode {
    value.parse().unwrap()
}

fn digest_bytes(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::from_bytes(Sha256::digest(bytes).into())
}

fn fake_digest(byte: u8) -> Sha256Digest {
    Sha256Digest::from_bytes([byte; 32])
}

fn base_project() -> ProjectV2 {
    let id = entity_id(1);
    let localization = Entity {
        id,
        display_name: "Greeting".into(),
        origin: OriginRef::New {
            authored_runtime_id: "loc:greeting".into(),
        },
        revision: 0,
        payload: EntityPayload::LocalizationEntry(LocalizationEntry {
            loc_id: "greeting".into(),
            texts: BTreeMap::from([(locale("de"), "Hallo".into())]),
        }),
    };
    ProjectV2 {
        format: FormatV2,
        schema_revision: SchemaRevisionV1,
        project_id: project_id(7),
        revision: 3,
        meta: ProjectMeta {
            name: "Store Fixture".into(),
            version: "1.2.3".into(),
            author: "tests".into(),
        },
        target: GameGenerationAnchor {
            executable: ContentSeal {
                byte_len: 123_456,
                sha256: fake_digest(0x42),
            },
        },
        authoring_locales: BTreeSet::from([locale("de")]),
        entities: BTreeMap::from([(id, localization)]),
        asset_store: AssetStoreIndex::default(),
    }
}

fn full_project(imported: &gore_authoring::ImportedOgg) -> ProjectV2 {
    let mut project = base_project();
    let localization_id = entity_id(1);
    let take_id = entity_id(2);
    let slot_id = entity_id(3);
    let line_id = entity_id(4);
    let authored_ref = |id, expected_kind| TypedRef::new(project.project_id, id, expected_kind);

    let take = Entity {
        id: take_id,
        display_name: "German take".into(),
        origin: OriginRef::Imported {
            importer: "working_store_import_ogg_v1".into(),
            source_seal: ContentSeal {
                byte_len: imported.asset.byte_len,
                sha256: imported.asset.sha256,
            },
            external_identity: None,
        },
        revision: 0,
        payload: EntityPayload::VoiceTake(VoiceTake {
            locale: locale("de"),
            asset: imported.asset.clone(),
            ogg: imported.ogg.clone(),
            status: VoiceTakeStatus::Approved,
        }),
    };
    let slot = Entity {
        id: slot_id,
        display_name: "German slot".into(),
        origin: OriginRef::New {
            authored_runtime_id: "voice-slot:greeting:de".into(),
        },
        revision: 0,
        payload: EntityPayload::VoiceSlot(VoiceSlot {
            locale: locale("de"),
            target_resolution: VoiceTargetResolution::Resolved {
                target: VoiceTarget {
                    archive: "german_new.zip".into(),
                    member: "NPC/Test/greeting.ogg".into(),
                    operation: VoiceOperation::Replace,
                    archive_seal: ArchiveSeal {
                        byte_len: 900,
                        sha256: fake_digest(0x33),
                    },
                    member_proof: VoiceMemberProof::Present {
                        uncompressed_size: 40,
                        crc32: 77,
                    },
                },
            },
            candidates: vec![authored_ref(take_id, EntityKind::VoiceTake)],
            selected: Some(authored_ref(take_id, EntityKind::VoiceTake)),
        }),
    };
    let line = Entity {
        id: line_id,
        display_name: "Greeting line".into(),
        origin: OriginRef::New {
            authored_runtime_id: "dialog-line:greeting".into(),
        },
        revision: 0,
        payload: EntityPayload::DialogLine(DialogLine {
            localization: authored_ref(localization_id, EntityKind::LocalizationEntry),
            speaker_hint: Some("test_npc".into()),
            voice_slots: BTreeMap::from([(
                locale("de"),
                authored_ref(slot_id, EntityKind::VoiceSlot),
            )]),
        }),
    };
    project.entities.insert(take_id, take);
    project.entities.insert(slot_id, slot);
    project.entities.insert(line_id, line);
    project.asset_store.assets.insert(
        imported.asset.sha256,
        AssetMeta {
            byte_len: imported.asset.byte_len,
            media_type: "audio/ogg".into(),
        },
    );
    project
}

fn store(root: &TestRoot) -> WorkingProjectStore {
    WorkingProjectStore::at(root.path(), WorkingStoreLimits::default()).unwrap()
}

fn publish(root: &TestRoot, head_bytes: &[u8]) {
    fs::write(root.path().join("gore-project.json"), head_bytes).unwrap();
}

fn digest_path(root: &Path, area: &str, digest: Sha256Digest, extension: &str) -> PathBuf {
    let hex = digest.to_string();
    root.join(area)
        .join("sha256")
        .join(&hex[..2])
        .join(format!("{}{}", &hex[2..], extension))
}

fn entity_path(root: &Path, id: EntityId, digest: Sha256Digest) -> PathBuf {
    let id_hex = id.to_string();
    root.join("entities")
        .join(&id_hex[..2])
        .join(&id_hex[2..])
        .join(format!("{digest}.json"))
}

fn write_candidate_snapshot(root: &Path, bytes: &[u8]) -> Vec<u8> {
    let sha256 = digest_bytes(bytes);
    let path = digest_path(root, "snapshots", sha256, ".json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
    serde_json::to_vec(&WorkingHead {
        store_format: Default::default(),
        snapshot: ContentSeal {
            byte_len: bytes.len() as u64,
            sha256,
        },
    })
    .unwrap()
}

fn replace_snapshot_entity_seal(
    snapshot: &[u8],
    id: EntityId,
    old: &ContentSeal,
    new: &ContentSeal,
) -> Vec<u8> {
    let old_member = format!("\"{id}\":{}", serde_json::to_string(old).unwrap());
    let new_member = format!("\"{id}\":{}", serde_json::to_string(new).unwrap());
    let snapshot = String::from_utf8(snapshot.to_vec()).unwrap();
    let replaced = snapshot.replacen(&old_member, &new_member, 1);
    assert_ne!(replaced, snapshot);
    replaced.into_bytes()
}

fn count_files(root: &Path) -> usize {
    fn visit(path: &Path, count: &mut usize) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                visit(&entry.path(), count);
            } else {
                *count += 1;
            }
        }
    }
    let mut count = 0;
    if root.exists() {
        visit(root, &mut count);
    }
    count
}

fn vorbis_ogg(sample_rate: u32) -> Vec<u8> {
    let mut data = include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg").to_vec();
    let ident = find_bytes(&data, b"\x01vorbis").expect("fixture has Vorbis identification");
    data[ident + 12..ident + 16].copy_from_slice(&sample_rate.to_le_bytes());
    rewrite_page_checksums(&mut data);
    data
}

fn opus_ogg(input_sample_rate: u32) -> Vec<u8> {
    let mut data = include_bytes!("../../gore-vo/testdata/tiny-opus.ogg").to_vec();
    let head = find_bytes(&data, b"OpusHead").expect("fixture has OpusHead");
    data[head + 12..head + 16].copy_from_slice(&input_sample_rate.to_le_bytes());
    rewrite_page_checksums(&mut data);
    data
}

fn opus_identification_only() -> Vec<u8> {
    let fixture = include_bytes!("../../gore-vo/testdata/tiny-opus.ogg");
    let segment_count = usize::from(fixture[26]);
    let header_len = 27 + segment_count;
    let body_len = fixture[27..header_len]
        .iter()
        .map(|value| usize::from(*value))
        .sum::<usize>();
    let mut data = fixture[..header_len + body_len].to_vec();
    data[5] |= 0x04;
    rewrite_page_checksums(&mut data);
    data
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn rewrite_page_checksums(data: &mut [u8]) {
    let mut offset = 0usize;
    while offset < data.len() {
        assert_eq!(&data[offset..offset + 4], b"OggS");
        let segment_count = usize::from(data[offset + 26]);
        let header_len = 27 + segment_count;
        let body_len = data[offset + 27..offset + header_len]
            .iter()
            .map(|value| usize::from(*value))
            .sum::<usize>();
        let page_len = header_len + body_len;
        data[offset + 22..offset + 26].fill(0);
        let checksum = ogg_crc(&data[offset..offset + page_len]);
        data[offset + 22..offset + 26].copy_from_slice(&checksum.to_le_bytes());
        offset += page_len;
    }
}

fn ogg_crc(page: &[u8]) -> u32 {
    let mut crc = 0u32;
    for (index, byte) in page.iter().copied().enumerate() {
        let byte = if (22..26).contains(&index) { 0 } else { byte };
        crc ^= u32::from(byte) << 24;
        for _ in 0..8 {
            crc = if crc & 0x8000_0000 != 0 {
                (crc << 1) ^ 0x04c1_1db7
            } else {
                crc << 1
            };
        }
    }
    crc
}

#[test]
fn deterministic_checkpoint_reopens_exactly_and_never_publishes_head() {
    let root = TestRoot::new("roundtrip");
    let store = store(&root);
    let project = base_project();

    let first = store
        .prepare_checkpoint(None, &project, ValidationProfile::Production)
        .unwrap();
    let second = store
        .prepare_checkpoint(None, &project, ValidationProfile::Production)
        .unwrap();

    assert_eq!(first, second);
    assert!(!first.blocks_build);
    assert!(first.diagnostics.is_empty());
    assert!(!root.path().join("gore-project.json").exists());
    let reopened = store
        .open_head_bytes(
            &first.head_bytes,
            AssetVerification::Full,
            ValidationProfile::Production,
        )
        .unwrap();
    assert_eq!(reopened.project, project);
    assert_eq!(reopened.head, first.head);

    let head_text = String::from_utf8(first.head_bytes).unwrap();
    assert_eq!(
        head_text,
        "{\"store_format\":1,\"snapshot\":{\"byte_len\":493,\"sha256\":\"31447eb3417ec5201ab28815738b8c7332f9f9e69ca58953ef59ba67e9282898\"}}"
    );
}

#[test]
fn changing_one_entity_only_adds_one_entity_shard() {
    let root = TestRoot::new("entity-delta");
    let store = store(&root);
    let project = base_project();
    store
        .prepare_checkpoint(None, &project, ValidationProfile::Production)
        .unwrap();
    let before = count_files(&root.path().join("entities"));

    let mut changed = project.clone();
    let entity = changed.entities.get_mut(&entity_id(1)).unwrap();
    entity.revision += 1;
    entity.display_name = "Changed greeting".into();
    store
        .prepare_checkpoint(None, &changed, ValidationProfile::Production)
        .unwrap();

    assert_eq!(count_files(&root.path().join("entities")), before + 1);
}

#[test]
fn valid_ogg_import_deduplicates_and_survives_source_deletion() {
    let root = TestRoot::new("ogg-dedup");
    let source = root.path().join("source.ogg");
    fs::write(&source, vorbis_ogg(48_000)).unwrap();
    let store = store(&root);

    let first = store
        .import_ogg(&source, "takes/greeting.ogg", None)
        .unwrap();
    assert!(!first.deduplicated);
    let second = store.import_ogg(&source, "second-name.ogg", None).unwrap();
    assert!(second.deduplicated);
    assert_eq!(first.asset.sha256, second.asset.sha256);
    assert_eq!(first.ogg.sample_rate, 48_000);

    fs::remove_file(source).unwrap();
    store
        .verify_asset(&first.asset, AssetVerification::Full)
        .unwrap();
    let asset_path = digest_path(root.path(), "assets", first.asset.sha256, "");
    assert_eq!(fs::read(asset_path).unwrap(), vorbis_ogg(48_000));
    assert_eq!(count_files(&root.path().join(".gore").join("staging")), 0);
}

#[test]
fn static_hardlink_alias_makes_an_immutable_blob_unsafe() {
    let root = TestRoot::new("hardlink-alias");
    let source = root.path().join("source.ogg");
    fs::write(&source, vorbis_ogg(48_000)).unwrap();
    let store = store(&root);
    let imported = store.import_ogg(&source, "take.ogg", None).unwrap();
    let asset_path = digest_path(root.path(), "assets", imported.asset.sha256, "");
    let alias = root.path().join("writable-alias.ogg");
    if fs::hard_link(&asset_path, &alias).is_ok() {
        assert!(matches!(
            store.verify_asset(&imported.asset, AssetVerification::Full),
            Err(WorkingStoreError::UnsafePath { .. })
        ));
        assert!(matches!(
            store.import_ogg(&source, "take.ogg", None),
            Err(WorkingStoreError::Collision { .. })
        ));
    }
}

#[test]
fn snapshot_and_entity_hardlink_aliases_are_rejected_on_reopen() {
    let root = TestRoot::new("manifest-hardlink-alias");
    let store = store(&root);
    let prepared = store
        .prepare_checkpoint(None, &base_project(), ValidationProfile::Production)
        .unwrap();
    let snapshot_path = digest_path(
        root.path(),
        "snapshots",
        prepared.head.snapshot.sha256,
        ".json",
    );
    let snapshot_alias = root.path().join("snapshot-alias.json");
    if fs::hard_link(&snapshot_path, &snapshot_alias).is_err() {
        return;
    }
    assert!(matches!(
        store.open_head_bytes(
            &prepared.head_bytes,
            AssetVerification::Full,
            ValidationProfile::Production
        ),
        Err(WorkingStoreError::UnsafePath { .. })
    ));
    fs::remove_file(snapshot_alias).unwrap();

    let snapshot: serde_json::Value =
        serde_json::from_slice(&fs::read(snapshot_path).unwrap()).unwrap();
    let id = entity_id(1);
    let seal: ContentSeal =
        serde_json::from_value(snapshot["entities"][id.to_string()].clone()).unwrap();
    let shard_path = entity_path(root.path(), id, seal.sha256);
    let shard_alias = root.path().join("entity-alias.json");
    fs::hard_link(shard_path, &shard_alias).unwrap();
    assert!(matches!(
        store.open_head_bytes(
            &prepared.head_bytes,
            AssetVerification::Full,
            ValidationProfile::Production
        ),
        Err(WorkingStoreError::UnsafePath { .. })
    ));
}

#[test]
fn opus_zero_input_rate_imports_with_the_fixed_decode_rate() {
    let root = TestRoot::new("opus-zero-rate");
    let source = root.path().join("source.ogg");
    fs::write(&source, opus_ogg(0)).unwrap();
    let imported = store(&root).import_ogg(&source, "opus.ogg", None).unwrap();
    assert_eq!(imported.ogg.codec, gore_authoring::OggCodec::Opus);
    assert_eq!(imported.ogg.sample_rate, 48_000);
    assert_eq!(imported.ogg.channels, 1);
}

#[test]
fn full_verification_rejects_spoofed_or_drifted_persisted_ogg_metadata() {
    let opus_root = TestRoot::new("opus-metadata-spoof");
    let opus_source = opus_root.path().join("source.ogg");
    fs::write(&opus_source, opus_ogg(48_000)).unwrap();
    let opus_store = store(&opus_root);
    let imported_opus = opus_store
        .import_ogg(&opus_source, "opus.ogg", None)
        .unwrap();
    let correct_opus = full_project(&imported_opus);
    let prepared = opus_store
        .prepare_checkpoint(None, &correct_opus, ValidationProfile::Experimental)
        .unwrap();

    let mut spoofed_opus = correct_opus.clone();
    let EntityPayload::VoiceTake(spoofed_take) = &mut spoofed_opus
        .entities
        .get_mut(&entity_id(2))
        .unwrap()
        .payload
    else {
        unreachable!();
    };
    spoofed_take.ogg.codec = OggCodec::Vorbis;
    assert!(matches!(
        opus_store.prepare_checkpoint(
            None,
            &spoofed_opus,
            ValidationProfile::Production
        ),
        Err(WorkingStoreError::OggMetadataMismatch {
            entity,
            declared: gore_authoring::OggMetadata {
                codec: OggCodec::Vorbis,
                ..
            },
            actual: gore_authoring::OggMetadata {
                codec: OggCodec::Opus,
                ..
            },
            ..
        }) if entity == entity_id(2)
    ));

    // Forge a canonical immutable entity/snapshot pair to prove Full reopen derives metadata from
    // the sealed asset instead of trusting a persisted VoiceTake payload.
    let snapshot_path = digest_path(
        opus_root.path(),
        "snapshots",
        prepared.head.snapshot.sha256,
        ".json",
    );
    let snapshot = fs::read(snapshot_path).unwrap();
    let snapshot_json: serde_json::Value = serde_json::from_slice(&snapshot).unwrap();
    let old_seal: ContentSeal =
        serde_json::from_value(snapshot_json["entities"][entity_id(2).to_string()].clone())
            .unwrap();
    let spoofed_entity = spoofed_opus.entities.get(&entity_id(2)).unwrap();
    let spoofed_bytes = serde_json::to_vec(spoofed_entity).unwrap();
    let spoofed_seal = ContentSeal {
        byte_len: spoofed_bytes.len() as u64,
        sha256: digest_bytes(&spoofed_bytes),
    };
    let spoofed_path = entity_path(opus_root.path(), entity_id(2), spoofed_seal.sha256);
    fs::create_dir_all(spoofed_path.parent().unwrap()).unwrap();
    fs::write(spoofed_path, &spoofed_bytes).unwrap();
    let spoofed_snapshot =
        replace_snapshot_entity_seal(&snapshot, entity_id(2), &old_seal, &spoofed_seal);
    let spoofed_head = write_candidate_snapshot(opus_root.path(), &spoofed_snapshot);
    assert!(matches!(
        opus_store.open_head_bytes(
            &spoofed_head,
            AssetVerification::Full,
            ValidationProfile::Production
        ),
        Err(WorkingStoreError::OggMetadataMismatch { entity, .. })
            if entity == entity_id(2)
    ));

    let vorbis_root = TestRoot::new("vorbis-metadata-drift");
    let vorbis_source = vorbis_root.path().join("source.ogg");
    fs::write(&vorbis_source, vorbis_ogg(44_100)).unwrap();
    let vorbis_store = store(&vorbis_root);
    let imported_vorbis = vorbis_store
        .import_ogg(&vorbis_source, "vorbis.ogg", None)
        .unwrap();
    let mut drifted_vorbis = full_project(&imported_vorbis);
    let EntityPayload::VoiceTake(drifted_take) = &mut drifted_vorbis
        .entities
        .get_mut(&entity_id(2))
        .unwrap()
        .payload
    else {
        unreachable!();
    };
    drifted_take.ogg.sample_rate = 48_000;
    assert!(matches!(
        vorbis_store.prepare_checkpoint(None, &drifted_vorbis, ValidationProfile::Production),
        Err(WorkingStoreError::OggMetadataMismatch {
            declared: gore_authoring::OggMetadata {
                sample_rate: 48_000,
                ..
            },
            actual: gore_authoring::OggMetadata {
                sample_rate: 44_100,
                ..
            },
            ..
        })
    ));
}

#[test]
fn fully_verified_opus_stays_draftable_but_never_silently_production_ready() {
    let root = TestRoot::new("verified-opus-gate");
    let source = root.path().join("source.ogg");
    fs::write(&source, opus_ogg(0)).unwrap();
    let store = store(&root);
    let imported = store.import_ogg(&source, "opus.ogg", None).unwrap();
    let project = full_project(&imported);

    let production = store
        .prepare_checkpoint(None, &project, ValidationProfile::Production)
        .unwrap();
    let production_gate = production
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagnosticCode::OpusDecodeUnproven)
        .unwrap();
    assert_eq!(production_gate.severity, DiagnosticSeverity::Error);
    assert!(production_gate.blocks_build);
    assert!(production.blocks_build);

    let experimental = store
        .prepare_checkpoint(None, &project, ValidationProfile::Experimental)
        .unwrap();
    let experimental_gate = experimental
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagnosticCode::OpusDecodeUnproven)
        .unwrap();
    assert_eq!(experimental_gate.severity, DiagnosticSeverity::Warning);
    assert!(!experimental_gate.blocks_build);
    assert!(!experimental.blocks_build);
}

#[test]
fn full_project_variant_round_trips_with_physical_asset_verification() {
    let root = TestRoot::new("full-project");
    let source = root.path().join("take.ogg");
    fs::write(&source, vorbis_ogg(44_100)).unwrap();
    let store = store(&root);
    let imported = store.import_ogg(&source, "greeting-de.ogg", None).unwrap();
    let project = full_project(&imported);
    assert!(project.validate().is_empty());

    let prepared = store
        .prepare_checkpoint(None, &project, ValidationProfile::Production)
        .unwrap();
    let reopened = store
        .open_head_bytes(
            &prepared.head_bytes,
            AssetVerification::Full,
            ValidationProfile::Production,
        )
        .unwrap();
    assert_eq!(reopened.project, project);
    assert!(!reopened.blocks_build);
}

#[test]
fn semantic_draft_diagnostics_are_saved_and_reported() {
    let root = TestRoot::new("draft");
    let store = store(&root);
    let mut project = base_project();
    let EntityPayload::LocalizationEntry(entry) =
        &mut project.entities.get_mut(&entity_id(1)).unwrap().payload
    else {
        unreachable!();
    };
    entry.texts.clear();

    let prepared = store
        .prepare_checkpoint(None, &project, ValidationProfile::Production)
        .unwrap();
    assert!(prepared.blocks_build);
    assert!(!prepared.diagnostics.is_empty());
    assert!(!root.path().join("gore-project.json").exists());
}

#[test]
fn missing_corrupt_hash_and_size_asset_fail_as_store_errors() {
    let root = TestRoot::new("asset-corruption");
    let source = root.path().join("take.ogg");
    fs::write(&source, vorbis_ogg(48_000)).unwrap();
    let store = store(&root);
    let imported = store.import_ogg(&source, "take.ogg", None).unwrap();
    let asset_path = digest_path(root.path(), "assets", imported.asset.sha256, "");

    fs::remove_file(&asset_path).unwrap();
    assert!(matches!(
        store.verify_asset(&imported.asset, AssetVerification::Full),
        Err(WorkingStoreError::MissingObject(_))
    ));

    let original = vorbis_ogg(48_000);
    fs::write(&asset_path, vec![0x55; original.len()]).unwrap();
    store
        .verify_asset(&imported.asset, AssetVerification::Structural)
        .unwrap();
    assert!(matches!(
        store.verify_asset(&imported.asset, AssetVerification::Full),
        Err(WorkingStoreError::SealMismatch { .. })
    ));

    fs::write(&asset_path, &original[..original.len() - 1]).unwrap();
    assert!(matches!(
        store.verify_asset(&imported.asset, AssetVerification::Structural),
        Err(WorkingStoreError::SealMismatch { .. })
    ));
}

#[test]
fn oversize_ogg_and_invalid_logical_names_are_rejected_before_install() {
    let root = TestRoot::new("ogg-limits");
    let source = root.path().join("take.ogg");
    fs::write(&source, vorbis_ogg(48_000)).unwrap();
    let limits = WorkingStoreLimits {
        max_ogg_bytes: 16,
        ..WorkingStoreLimits::default()
    };
    let store = WorkingProjectStore::at(root.path(), limits).unwrap();

    assert!(matches!(
        store.import_ogg(&source, "take.ogg", None),
        Err(WorkingStoreError::LimitExceeded {
            kind: "Ogg bytes",
            ..
        })
    ));

    let store = WorkingProjectStore::at(root.path(), WorkingStoreLimits::default()).unwrap();
    assert!(matches!(
        store.import_ogg(&source, "bad\nname.ogg", None),
        Err(WorkingStoreError::Invariant(_))
    ));
    assert!(matches!(
        store.import_ogg(&source, "x".repeat(1025), None),
        Err(WorkingStoreError::LimitExceeded { .. })
    ));
    assert_eq!(count_files(&root.path().join("assets")), 0);
}

#[test]
fn invalid_ogg_and_hostile_staging_objects_never_create_or_replace_asset_blobs() {
    let invalid_root = TestRoot::new("invalid-before-install");
    let source = invalid_root.path().join("invalid.ogg");
    fs::write(&source, b"not an ogg stream").unwrap();
    let invalid_store = store(&invalid_root);
    assert!(matches!(
        invalid_store.import_ogg(&source, "invalid.ogg", None),
        Err(WorkingStoreError::InvalidOgg(_))
    ));
    assert_eq!(count_files(&invalid_root.path().join("assets")), 0);

    fs::write(&source, opus_identification_only()).unwrap();
    assert!(matches!(
        invalid_store.import_ogg(&source, "header-only.ogg", None),
        Err(WorkingStoreError::InvalidOgg(_))
    ));
    assert_eq!(count_files(&invalid_root.path().join("assets")), 0);

    let hostile_root = TestRoot::new("hostile-staging");
    let source = hostile_root.path().join("valid.ogg");
    fs::write(&source, vorbis_ogg(48_000)).unwrap();
    let sentinel = b"do not replace staging owner";
    fs::write(hostile_root.path().join(".gore"), sentinel).unwrap();
    let hostile_store = store(&hostile_root);
    assert!(hostile_store
        .import_ogg(&source, "valid.ogg", None)
        .is_err());
    assert_eq!(
        fs::read(hostile_root.path().join(".gore")).unwrap(),
        sentinel
    );
    assert_eq!(count_files(&hostile_root.path().join("assets")), 0);

    let linked_root = TestRoot::new("linked-staging");
    let source = linked_root.path().join("valid.ogg");
    fs::write(&source, vorbis_ogg(48_000)).unwrap();
    let outside = linked_root.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::create_dir_all(linked_root.path().join(".gore")).unwrap();
    let staging = linked_root.path().join(".gore").join("staging");
    if create_dir_symlink(&outside, &staging).is_ok() {
        let store = store(&linked_root);
        assert!(matches!(
            store.import_ogg(&source, "valid.ogg", None),
            Err(WorkingStoreError::UnsafePath { .. })
        ));
        assert_eq!(count_files(&outside), 0);
        assert_eq!(count_files(&linked_root.path().join("assets")), 0);
    }
}

#[test]
fn unknown_duplicate_and_noncanonical_head_json_are_rejected() {
    let root = TestRoot::new("strict-head");
    let store = store(&root);
    let prepared = store
        .prepare_checkpoint(None, &base_project(), ValidationProfile::Production)
        .unwrap();
    let canonical = String::from_utf8(prepared.head_bytes).unwrap();
    let unknown = canonical.replacen(
        "{\"store_format\":1,",
        "{\"store_format\":1,\"unknown\":true,",
        1,
    );
    assert!(matches!(
        store.open_head_bytes(
            unknown.as_bytes(),
            AssetVerification::Full,
            ValidationProfile::Production
        ),
        Err(WorkingStoreError::InvalidJson { .. })
    ));
    let duplicate = canonical.replacen(
        "{\"store_format\":1,",
        "{\"store_format\":1,\"store_format\":1,",
        1,
    );
    assert!(matches!(
        store.open_head_bytes(
            duplicate.as_bytes(),
            AssetVerification::Full,
            ValidationProfile::Production
        ),
        Err(WorkingStoreError::InvalidJson { .. })
    ));
    let spaced = format!(" {canonical}");
    assert!(matches!(
        store.open_head_bytes(
            spaced.as_bytes(),
            AssetVerification::Full,
            ValidationProfile::Production
        ),
        Err(WorkingStoreError::NonCanonicalJson { kind: "head" })
    ));
}

#[test]
fn unknown_duplicate_and_noncanonical_snapshot_json_are_rejected() {
    let root = TestRoot::new("strict-snapshot");
    let store = store(&root);
    let prepared = store
        .prepare_checkpoint(None, &base_project(), ValidationProfile::Production)
        .unwrap();
    let snapshot_path = digest_path(
        root.path(),
        "snapshots",
        prepared.head.snapshot.sha256,
        ".json",
    );
    let canonical = String::from_utf8(fs::read(snapshot_path).unwrap()).unwrap();

    let unknown = canonical.replacen(
        "{\"store_format\":1,",
        "{\"store_format\":1,\"unknown\":true,",
        1,
    );
    let head = write_candidate_snapshot(root.path(), unknown.as_bytes());
    assert!(matches!(
        store.open_head_bytes(
            &head,
            AssetVerification::Full,
            ValidationProfile::Production
        ),
        Err(WorkingStoreError::InvalidJson {
            kind: "snapshot",
            ..
        })
    ));

    let duplicate = canonical.replacen(
        "{\"store_format\":1,",
        "{\"store_format\":1,\"store_format\":1,",
        1,
    );
    let head = write_candidate_snapshot(root.path(), duplicate.as_bytes());
    assert!(matches!(
        store.open_head_bytes(
            &head,
            AssetVerification::Full,
            ValidationProfile::Production
        ),
        Err(WorkingStoreError::InvalidJson {
            kind: "snapshot",
            ..
        })
    ));

    let spaced = format!(" {canonical}");
    let head = write_candidate_snapshot(root.path(), spaced.as_bytes());
    assert!(matches!(
        store.open_head_bytes(
            &head,
            AssetVerification::Full,
            ValidationProfile::Production
        ),
        Err(WorkingStoreError::NonCanonicalJson { kind: "snapshot" })
    ));
}

#[test]
fn duplicate_entity_index_keys_and_strict_entity_shards_are_rejected() {
    let root = TestRoot::new("strict-entity");
    let store = store(&root);
    let prepared = store
        .prepare_checkpoint(None, &base_project(), ValidationProfile::Production)
        .unwrap();
    let snapshot_path = digest_path(
        root.path(),
        "snapshots",
        prepared.head.snapshot.sha256,
        ".json",
    );
    let snapshot = fs::read(&snapshot_path).unwrap();
    let snapshot_value: serde_json::Value = serde_json::from_slice(&snapshot).unwrap();
    let id = entity_id(1);
    let old_seal: ContentSeal =
        serde_json::from_value(snapshot_value["entities"][id.to_string()].clone()).unwrap();
    let old_member = format!("\"{id}\":{}", serde_json::to_string(&old_seal).unwrap());
    let duplicate_index = String::from_utf8(snapshot.clone()).unwrap().replacen(
        &old_member,
        &format!("{old_member},{old_member}"),
        1,
    );
    let head = write_candidate_snapshot(root.path(), duplicate_index.as_bytes());
    assert!(matches!(
        store.open_head_bytes(
            &head,
            AssetVerification::Full,
            ValidationProfile::Production
        ),
        Err(WorkingStoreError::InvalidJson {
            kind: "snapshot",
            ..
        })
    ));

    let shard_path = entity_path(root.path(), id, old_seal.sha256);
    let canonical_entity = String::from_utf8(fs::read(shard_path).unwrap()).unwrap();
    for hostile in [
        canonical_entity.replacen("{\"id\":", "{\"unknown\":true,\"id\":", 1),
        canonical_entity.replacen("{\"id\":", &format!("{{\"id\":\"{id}\",\"id\":"), 1),
    ] {
        let seal = ContentSeal {
            byte_len: hostile.len() as u64,
            sha256: digest_bytes(hostile.as_bytes()),
        };
        let path = entity_path(root.path(), id, seal.sha256);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, hostile.as_bytes()).unwrap();
        let candidate_snapshot = replace_snapshot_entity_seal(&snapshot, id, &old_seal, &seal);
        let head = write_candidate_snapshot(root.path(), &candidate_snapshot);
        assert!(matches!(
            store.open_head_bytes(
                &head,
                AssetVerification::Full,
                ValidationProfile::Production
            ),
            Err(WorkingStoreError::InvalidJson { kind: "entity", .. })
        ));
    }
}

#[test]
fn missing_and_corrupt_snapshot_or_entity_objects_are_hard_failures() {
    let root = TestRoot::new("manifest-corruption");
    let store = store(&root);
    let prepared = store
        .prepare_checkpoint(None, &base_project(), ValidationProfile::Production)
        .unwrap();
    let snapshot_path = digest_path(
        root.path(),
        "snapshots",
        prepared.head.snapshot.sha256,
        ".json",
    );
    let snapshot = fs::read(&snapshot_path).unwrap();
    fs::remove_file(&snapshot_path).unwrap();
    assert!(matches!(
        store.open_head_bytes(
            &prepared.head_bytes,
            AssetVerification::Full,
            ValidationProfile::Production
        ),
        Err(WorkingStoreError::MissingObject(_))
    ));

    fs::write(&snapshot_path, vec![0; snapshot.len()]).unwrap();
    assert!(matches!(
        store.open_head_bytes(
            &prepared.head_bytes,
            AssetVerification::Full,
            ValidationProfile::Production
        ),
        Err(WorkingStoreError::SealMismatch { .. })
    ));

    fs::write(&snapshot_path, &snapshot).unwrap();
    let snapshot_json: serde_json::Value = serde_json::from_slice(&snapshot).unwrap();
    let entity_seal: ContentSeal =
        serde_json::from_value(snapshot_json["entities"][entity_id(1).to_string()].clone())
            .unwrap();
    let shard_path = entity_path(root.path(), entity_id(1), entity_seal.sha256);
    let shard = fs::read(&shard_path).unwrap();
    fs::write(&shard_path, vec![0; shard.len()]).unwrap();
    assert!(matches!(
        store.open_head_bytes(
            &prepared.head_bytes,
            AssetVerification::Full,
            ValidationProfile::Production
        ),
        Err(WorkingStoreError::SealMismatch { .. })
    ));
}

#[test]
fn corrupt_existing_digest_target_is_a_collision_not_a_replacement() {
    let root = TestRoot::new("collision");
    let source = root.path().join("take.ogg");
    let ogg = vorbis_ogg(48_000);
    fs::write(&source, &ogg).unwrap();
    let store = store(&root);
    let imported = store.import_ogg(&source, "take.ogg", None).unwrap();
    let asset_path = digest_path(root.path(), "assets", imported.asset.sha256, "");
    fs::write(&asset_path, vec![0x7f; ogg.len()]).unwrap();

    assert!(matches!(
        store.import_ogg(&source, "take.ogg", None),
        Err(WorkingStoreError::Collision { .. })
    ));
    assert_eq!(fs::read(asset_path).unwrap(), vec![0x7f; ogg.len()]);
}

#[test]
fn expected_head_conflict_does_not_touch_fixed_head() {
    let root = TestRoot::new("head-conflict");
    let store = store(&root);
    let first = store
        .prepare_checkpoint(None, &base_project(), ValidationProfile::Production)
        .unwrap();
    publish(&root, &first.head_bytes);
    let fixed_before = fs::read(root.path().join("gore-project.json")).unwrap();
    let wrong = WorkingHead {
        store_format: Default::default(),
        snapshot: ContentSeal {
            byte_len: 1,
            sha256: fake_digest(0x99),
        },
    };

    let mut changed = base_project();
    changed.revision += 1;
    assert!(matches!(
        store.prepare_checkpoint(Some(&wrong), &changed, ValidationProfile::Production),
        Err(WorkingStoreError::HeadConflict { .. })
    ));
    assert_eq!(
        fs::read(root.path().join("gore-project.json")).unwrap(),
        fixed_before
    );
    assert_eq!(store.current_head().unwrap(), Some(first.head));
}

#[test]
fn absent_expected_head_conflicts_with_an_existing_fixed_head() {
    let root = TestRoot::new("absent-head-conflict");
    let store = store(&root);
    let first = store
        .prepare_checkpoint(None, &base_project(), ValidationProfile::Production)
        .unwrap();
    publish(&root, &first.head_bytes);
    let fixed_before = fs::read(root.path().join("gore-project.json")).unwrap();

    assert!(matches!(
        store.prepare_checkpoint(None, &base_project(), ValidationProfile::Production),
        Err(WorkingStoreError::HeadConflict {
            expected: None,
            actual: Some(_)
        })
    ));
    assert_eq!(
        fs::read(root.path().join("gore-project.json")).unwrap(),
        fixed_before
    );
}

#[test]
fn fixed_head_open_uses_the_same_strict_reconstitution_path() {
    let root = TestRoot::new("fixed-open");
    let store = store(&root);
    let project = base_project();
    let prepared = store
        .prepare_checkpoint(None, &project, ValidationProfile::Experimental)
        .unwrap();
    publish(&root, &prepared.head_bytes);

    assert_eq!(store.current_head().unwrap(), Some(prepared.head.clone()));
    let opened = store
        .open_current(AssetVerification::Full, ValidationProfile::Experimental)
        .unwrap();
    assert_eq!(opened.project, project);
}

#[test]
fn configured_store_limits_must_be_finite_and_within_format_caps() {
    let root = TestRoot::new("limit-config");
    let zero = WorkingStoreLimits {
        max_entities: 0,
        ..WorkingStoreLimits::default()
    };
    assert!(matches!(
        WorkingProjectStore::at(root.path(), zero),
        Err(WorkingStoreError::InvalidLimits(_))
    ));

    let too_large = WorkingStoreLimits {
        max_head_bytes: 64 * 1024 + 1,
        ..WorkingStoreLimits::default()
    };
    assert!(matches!(
        WorkingProjectStore::at(root.path(), too_large),
        Err(WorkingStoreError::InvalidLimits(_))
    ));
}

#[test]
fn open_existing_never_creates_a_missing_root_or_parent() {
    let root = TestRoot::new("open-existing-missing");
    let missing_parent = root.path().join("missing-parent");
    let missing_root = missing_parent.join("store");

    assert!(matches!(
        WorkingProjectStore::open_existing(&missing_root, WorkingStoreLimits::default()),
        Err(WorkingStoreError::MissingRoot(path)) if path == missing_root
    ));
    assert!(!missing_parent.exists());
    assert!(!missing_root.exists());
}

#[test]
fn open_existing_accepts_a_real_root_and_rejects_a_link_root() {
    let root = TestRoot::new("open-existing-real");
    let existing = root.path().join("existing-store");
    fs::create_dir(&existing).unwrap();
    let opened =
        WorkingProjectStore::open_existing(&existing, WorkingStoreLimits::default()).unwrap();
    assert_eq!(opened.root(), existing);

    let linked = root.path().join("linked-store");
    if create_dir_symlink(&existing, &linked).is_ok() {
        assert!(matches!(
            WorkingProjectStore::open_existing(&linked, WorkingStoreLimits::default()),
            Err(WorkingStoreError::UnsafePath { .. })
        ));
    }
}

#[test]
fn head_snapshot_entity_count_asset_count_and_aggregate_limits_are_enforced() {
    let root = TestRoot::new("all-limits");
    let primary_store = store(&root);
    let oversized_head = vec![b' '; 64 * 1024 + 1];
    assert!(matches!(
        primary_store.open_head_bytes(
            &oversized_head,
            AssetVerification::Full,
            ValidationProfile::Production
        ),
        Err(WorkingStoreError::LimitExceeded {
            kind: "head bytes",
            ..
        })
    ));

    let oversized_snapshot_head = serde_json::to_vec(&WorkingHead {
        store_format: Default::default(),
        snapshot: ContentSeal {
            byte_len: 16 * 1024 * 1024 + 1,
            sha256: fake_digest(1),
        },
    })
    .unwrap();
    assert!(matches!(
        primary_store.open_head_bytes(
            &oversized_snapshot_head,
            AssetVerification::Full,
            ValidationProfile::Production
        ),
        Err(WorkingStoreError::LimitExceeded {
            kind: "snapshot",
            ..
        })
    ));

    let entity_limited = WorkingProjectStore::at(
        root.path().join("entity-limited"),
        WorkingStoreLimits {
            max_entity_bytes: 32,
            ..WorkingStoreLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        entity_limited.prepare_checkpoint(None, &base_project(), ValidationProfile::Production),
        Err(WorkingStoreError::LimitExceeded {
            kind: "entity bytes",
            ..
        })
    ));

    let snapshot_limited_root = root.path().join("snapshot-limited");
    let snapshot_limited = WorkingProjectStore::at(
        &snapshot_limited_root,
        WorkingStoreLimits {
            max_snapshot_bytes: 64,
            ..WorkingStoreLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        snapshot_limited.prepare_checkpoint(None, &base_project(), ValidationProfile::Production),
        Err(WorkingStoreError::LimitExceeded {
            kind: "snapshot bytes",
            ..
        })
    ));
    // The already complete entity object is merely an orphan; no partial snapshot is installed.
    assert_eq!(count_files(&snapshot_limited_root.join("snapshots")), 0);
    assert_eq!(count_files(&snapshot_limited_root.join("entities")), 1);

    let mut two_entities = base_project();
    let mut second = two_entities.entities[&entity_id(1)].clone();
    second.id = entity_id(2);
    two_entities.entities.insert(second.id, second);
    let entity_count_store = WorkingProjectStore::at(
        root.path().join("entity-count"),
        WorkingStoreLimits {
            max_entities: 1,
            ..WorkingStoreLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        entity_count_store.prepare_checkpoint(None, &two_entities, ValidationProfile::Production),
        Err(WorkingStoreError::LimitExceeded {
            kind: "entity count",
            ..
        })
    ));

    let mut two_assets = base_project();
    two_assets.asset_store.assets.insert(
        fake_digest(2),
        AssetMeta {
            byte_len: 1,
            media_type: "application/octet-stream".into(),
        },
    );
    two_assets.asset_store.assets.insert(
        fake_digest(3),
        AssetMeta {
            byte_len: 1,
            media_type: "application/octet-stream".into(),
        },
    );
    let asset_count_store = WorkingProjectStore::at(
        root.path().join("asset-count"),
        WorkingStoreLimits {
            max_assets: 1,
            ..WorkingStoreLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        asset_count_store.prepare_checkpoint(None, &two_assets, ValidationProfile::Production),
        Err(WorkingStoreError::LimitExceeded {
            kind: "asset count",
            ..
        })
    ));

    let mut aggregate = base_project();
    aggregate.asset_store.assets.insert(
        fake_digest(4),
        AssetMeta {
            byte_len: 11,
            media_type: "application/octet-stream".into(),
        },
    );
    let aggregate_store = WorkingProjectStore::at(
        root.path().join("asset-aggregate"),
        WorkingStoreLimits {
            max_referenced_asset_bytes: 10,
            ..WorkingStoreLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        aggregate_store.prepare_checkpoint(None, &aggregate, ValidationProfile::Production),
        Err(WorkingStoreError::LimitExceeded {
            kind: "aggregate referenced asset bytes",
            ..
        })
    ));

    let project = base_project();
    let entity_bytes = serde_json::to_vec(&project.entities[&entity_id(1)])
        .unwrap()
        .len() as u64;
    let exact_entity_store = WorkingProjectStore::at(
        root.path().join("entity-aggregate-exact"),
        WorkingStoreLimits {
            max_referenced_entity_bytes: entity_bytes,
            ..WorkingStoreLimits::default()
        },
    )
    .unwrap();
    exact_entity_store
        .prepare_checkpoint(None, &project, ValidationProfile::Production)
        .unwrap();

    let short_entity_store = WorkingProjectStore::at(
        root.path().join("entity-aggregate-short"),
        WorkingStoreLimits {
            max_referenced_entity_bytes: entity_bytes - 1,
            ..WorkingStoreLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        short_entity_store.prepare_checkpoint(None, &project, ValidationProfile::Production),
        Err(WorkingStoreError::LimitExceeded {
            kind: "aggregate referenced entity bytes",
            ..
        })
    ));

    let reopen_root = TestRoot::new("entity-aggregate-reopen");
    let prepared = store(&reopen_root)
        .prepare_checkpoint(None, &project, ValidationProfile::Production)
        .unwrap();
    let constrained_reopen = WorkingProjectStore::at(
        reopen_root.path(),
        WorkingStoreLimits {
            max_referenced_entity_bytes: entity_bytes - 1,
            ..WorkingStoreLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        constrained_reopen.open_head_bytes(
            &prepared.head_bytes,
            AssetVerification::Full,
            ValidationProfile::Production
        ),
        Err(WorkingStoreError::LimitExceeded {
            kind: "aggregate referenced entity bytes",
            ..
        })
    ));
}

#[test]
fn phase_one_ogg_references_obey_the_per_blob_64_mib_ceiling() {
    let root = TestRoot::new("indexed-ogg-limit");
    let store = store(&root);
    let mut project = base_project();
    project.asset_store.assets.insert(
        fake_digest(8),
        AssetMeta {
            byte_len: 64 * 1024 * 1024 + 1,
            media_type: "audio/ogg".into(),
        },
    );
    assert!(matches!(
        store.prepare_checkpoint(None, &project, ValidationProfile::Production),
        Err(WorkingStoreError::LimitExceeded {
            kind: "Ogg bytes",
            ..
        })
    ));
    let oversized_ref = gore_authoring::AssetRef {
        sha256: fake_digest(9),
        byte_len: 64 * 1024 * 1024 + 1,
        logical_name: "too-large.ogg".into(),
    };
    assert!(matches!(
        store.verify_asset(&oversized_ref, AssetVerification::Structural),
        Err(WorkingStoreError::LimitExceeded {
            kind: "Ogg bytes",
            ..
        })
    ));
}

#[test]
fn entity_aggregate_limit_precedes_missing_asset_filesystem_work() {
    let root = TestRoot::new("manifest-check-order");
    let initial_store = store(&root);
    let prepared = initial_store
        .prepare_checkpoint(None, &base_project(), ValidationProfile::Production)
        .unwrap();
    let snapshot_path = digest_path(
        root.path(),
        "snapshots",
        prepared.head.snapshot.sha256,
        ".json",
    );
    let snapshot = fs::read(&snapshot_path).unwrap();
    let snapshot_value: serde_json::Value = serde_json::from_slice(&snapshot).unwrap();
    let entity_seal: ContentSeal =
        serde_json::from_value(snapshot_value["entities"][entity_id(1).to_string()].clone())
            .unwrap();

    let missing_digest = fake_digest(0xa5);
    let with_missing_asset = String::from_utf8(snapshot)
        .unwrap()
        .replacen(
            "\"asset_store\":{\"assets\":{}}",
            &format!(
                "\"asset_store\":{{\"assets\":{{\"{missing_digest}\":{{\"byte_len\":1,\"media_type\":\"application/octet-stream\"}}}}}}"
            ),
            1,
        );
    assert!(with_missing_asset.contains(&missing_digest.to_string()));
    let candidate_head = write_candidate_snapshot(root.path(), with_missing_asset.as_bytes());
    let constrained = WorkingProjectStore::at(
        root.path(),
        WorkingStoreLimits {
            max_referenced_entity_bytes: entity_seal.byte_len - 1,
            ..WorkingStoreLimits::default()
        },
    )
    .unwrap();

    assert!(matches!(
        constrained.open_head_bytes(
            &candidate_head,
            AssetVerification::Full,
            ValidationProfile::Production
        ),
        Err(WorkingStoreError::LimitExceeded {
            kind: "aggregate referenced entity bytes",
            ..
        })
    ));
}

#[test]
fn indexed_but_missing_physical_asset_blocks_checkpoint_before_objects_are_written() {
    let root = TestRoot::new("missing-indexed-asset");
    let store = store(&root);
    let mut project = base_project();
    project.asset_store.assets.insert(
        fake_digest(7),
        AssetMeta {
            byte_len: 20,
            media_type: "application/octet-stream".into(),
        },
    );
    assert!(matches!(
        store.prepare_checkpoint(None, &project, ValidationProfile::Production),
        Err(WorkingStoreError::MissingObject(_))
    ));
    assert_eq!(count_files(&root.path().join("entities")), 0);
    assert_eq!(count_files(&root.path().join("snapshots")), 0);
}

#[test]
fn symlinked_root_and_source_are_rejected_when_platform_allows_creation() {
    let root = TestRoot::new("links");
    let real_store = root.path().join("real-store");
    fs::create_dir_all(&real_store).unwrap();
    let linked_store = root.path().join("linked-store");
    if create_dir_symlink(&real_store, &linked_store).is_ok() {
        assert!(matches!(
            WorkingProjectStore::at(&linked_store, WorkingStoreLimits::default()),
            Err(WorkingStoreError::UnsafePath { .. })
        ));
    }

    let safe_store = WorkingProjectStore::at(
        root.path().join("safe-store"),
        WorkingStoreLimits::default(),
    )
    .unwrap();
    let source = root.path().join("source.ogg");
    fs::write(&source, vorbis_ogg(48_000)).unwrap();
    let linked_source = root.path().join("linked-source.ogg");
    if create_file_symlink(&source, &linked_source).is_ok() {
        assert!(matches!(
            safe_store.import_ogg(&linked_source, "take.ogg", None),
            Err(WorkingStoreError::UnsafePath { .. })
        ));
    }
}

#[cfg(windows)]
fn create_dir_symlink(original: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(original, link)
}

#[cfg(not(windows))]
fn create_dir_symlink(original: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(original, link)
}

#[cfg(windows)]
fn create_file_symlink(original: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(original, link)
}

#[cfg(not(windows))]
fn create_file_symlink(original: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(original, link)
}

#[test]
fn snapshot_object_path_uses_the_sealed_sha256_layout() {
    let root = TestRoot::new("layout");
    let store = store(&root);
    let prepared = store
        .prepare_checkpoint(None, &base_project(), ValidationProfile::Production)
        .unwrap();
    let snapshot = digest_path(
        root.path(),
        "snapshots",
        prepared.head.snapshot.sha256,
        ".json",
    );
    assert!(snapshot.is_file());
    assert_eq!(
        fs::metadata(snapshot).unwrap().len(),
        prepared.head.snapshot.byte_len
    );
}

#[test]
fn hash_helper_matches_imported_asset_seal() {
    let root = TestRoot::new("hash-layout");
    let source = root.path().join("source.ogg");
    let bytes = vorbis_ogg(32_000);
    fs::write(&source, &bytes).unwrap();
    let imported = store(&root).import_ogg(&source, "take.ogg", None).unwrap();
    assert_eq!(imported.asset.sha256, digest_bytes(&bytes));
    assert_eq!(imported.asset.byte_len, bytes.len() as u64);
}
