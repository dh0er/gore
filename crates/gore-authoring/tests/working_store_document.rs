use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use gore_authoring::model_revision2::{
    Entity as Revision2Entity, EntityKind as Revision2EntityKind,
    EntityPayload as Revision2EntityPayload, NpcDraft, NpcDraftInput, NpcParentClassInput,
    OggCodec as Revision2OggCodec, OggMetadata as Revision2OggMetadata,
    OriginRef as Revision2OriginRef, ProjectRevision2, QuestCollisionCatalogInput, QuestDraft,
    QuestDraftInput, QuestGiverInput, QuestParentInput, SchemaRevisionV2, ScriptModule,
    TypedRef as Revision2TypedRef, VoiceTake as Revision2VoiceTake,
    VoiceTakeStatus as Revision2VoiceTakeStatus,
};
use gore_authoring::{
    AssetMeta, AssetStoreIndex, AssetVerification, ContentSeal, DiagnosticCode, DiagnosticSeverity,
    Entity, EntityId, EntityPayload, FormatV2, GameGenerationAnchor, LocaleCode, LocalizationEntry,
    OriginRef, ProjectDocument, ProjectId, ProjectMeta, ProjectV2, SchemaRevisionV1, Sha256Digest,
    ValidationProfile, WorkingHead, WorkingProjectStore, WorkingStoreError, WorkingStoreLimits,
    DRAFT_QUEST_GENERATOR_ID, DRAFT_QUEST_GENERATOR_VERSION, LOGICAL_NPC_CLONE_GENERATOR_ID,
    LOGICAL_NPC_CLONE_GENERATOR_VERSION,
};
use sha2::{Digest, Sha256};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);

impl TestRoot {
    fn new(label: &str) -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "gore-authoring-document-store-{label}-{}-{sequence}",
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

fn entity_id(value: u8) -> EntityId {
    EntityId::from_bytes([value; 16])
}

fn project_id(value: u8) -> ProjectId {
    ProjectId::from_bytes([value; 16])
}

fn locale(value: &str) -> LocaleCode {
    value.parse().unwrap()
}

fn seal(value: u8, byte_len: u64) -> ContentSeal {
    ContentSeal {
        byte_len,
        sha256: Sha256Digest::from_bytes([value; 32]),
    }
}

fn generation(value: u8) -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: seal(value, 1_000_000),
    }
}

fn store(root: &TestRoot) -> WorkingProjectStore {
    WorkingProjectStore::at(root.path(), WorkingStoreLimits::default()).unwrap()
}

fn revision1_project() -> ProjectV2 {
    let id: EntityId = "00000000000000000000000000000001".parse().unwrap();
    ProjectV2 {
        format: FormatV2,
        schema_revision: SchemaRevisionV1,
        project_id: "00000000000000000000000000000007".parse().unwrap(),
        revision: 3,
        meta: ProjectMeta {
            name: "Store Fixture".into(),
            version: "1.2.3".into(),
            author: "tests".into(),
        },
        target: GameGenerationAnchor {
            executable: ContentSeal {
                byte_len: 123_456,
                sha256: Sha256Digest::from_bytes([0x42; 32]),
            },
        },
        authoring_locales: BTreeSet::from([locale("de")]),
        entities: BTreeMap::from([(
            id,
            Entity {
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
            },
        )]),
        asset_store: AssetStoreIndex::default(),
    }
}

fn npc_parent(
    target: &GameGenerationAnchor,
    seal_byte: u8,
    selector: &str,
    runtime_class: &str,
) -> NpcParentClassInput {
    NpcParentClassInput {
        generation: target.clone(),
        source_seal: seal(seal_byte, 20_000),
        catalog_layer: "base-game.g1r.characters".into(),
        canonical_selector: selector.into(),
        runtime_class: runtime_class.into(),
    }
}

fn npc_draft(project: ProjectId, target: &GameGenerationAnchor) -> NpcDraft {
    NpcDraft {
        generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.into(),
        generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
        input: NpcDraftInput {
            target: target.clone(),
            module_namespace: "GoreMods.Npcs.AsghanClone".into(),
            unique_name: "GoreAsghanClone".into(),
            parent_character_definition: npc_parent(
                target,
                2,
                "CatalogCharacterDefinition_Asghan",
                "UCharacterDefinition_Human_Asghan",
            ),
            parent_ai_agent_config: npc_parent(
                target,
                3,
                "CatalogAiAgentConfig_Asghan",
                "UAIAgentConfig_Human_Asghan",
            ),
            parent_spawn_definition: npc_parent(
                target,
                4,
                "CatalogSpawnDefinition_Asghan",
                "USpawnAIAgentDefinition_Asghan",
            ),
        },
        script_module: Revision2TypedRef::new(
            project,
            entity_id(11),
            Revision2EntityKind::ScriptModule,
        ),
    }
}

fn quest_draft(project: ProjectId, target: &GameGenerationAnchor) -> QuestDraft {
    QuestDraft {
        generator_id: DRAFT_QUEST_GENERATOR_ID.into(),
        generator_version: DRAFT_QUEST_GENERATOR_VERSION,
        input: QuestDraftInput {
            target: target.clone(),
            quest_id: entity_id(20),
            module_namespace: "GoreMods.Quests.AsghanTrial".into(),
            technical_id: "GORE_ASGHAN_TRIAL".into(),
            text_helper: "GoreAsghanTrialText".into(),
            parent_quest: QuestParentInput {
                generation: target.clone(),
                source_seal: seal(5, 30_000),
                catalog_layer: "base-game.g1r.quests".into(),
                canonical_selector: "CatalogQuest_AsghanParent".into(),
                runtime_class: "UQuest_SwampCamp_SCCHAPTER2".into(),
            },
            giver: QuestGiverInput {
                generation: target.clone(),
                source_seal: seal(6, 40_000),
                catalog_layer: "base-game.g1r.characters".into(),
                canonical_selector: "CatalogCharacter_Asghan".into(),
                runtime_unique_name: "OM_GRD_Asghan_263".into(),
            },
            title: "Asghan's Trial".into(),
            description: "Prove that the gate is secure.".into(),
            objective_title: "Report to Asghan".into(),
            collision_catalog: QuestCollisionCatalogInput {
                generation: target.clone(),
                source_seal: seal(7, 50_000),
                catalog_layer: "resolved-loadout.scripts.v1".into(),
                modules: BTreeSet::from(["existing.module".into()]),
                relative_paths: BTreeSet::from(["existing/module.as".into()]),
                symbols: BTreeSet::from(["uexistingsymbol".into()]),
            },
        },
        script_module: Revision2TypedRef::new(
            project,
            entity_id(21),
            Revision2EntityKind::ScriptModule,
        ),
    }
}

fn story_project() -> ProjectRevision2 {
    let project_id = project_id(1);
    let target = generation(1);
    let npc = npc_draft(project_id, &target);
    let quest = quest_draft(project_id, &target);
    let npc_owner =
        Revision2TypedRef::new(project_id, entity_id(10), Revision2EntityKind::NpcDraft);
    let quest_owner =
        Revision2TypedRef::new(project_id, entity_id(20), Revision2EntityKind::QuestDraft);
    let npc_script = npc.regenerate_script_module(npc_owner.clone()).unwrap();
    let quest_script = quest.regenerate_script_module(quest_owner.clone()).unwrap();

    let entities = BTreeMap::from([
        (
            entity_id(10),
            Revision2Entity {
                id: entity_id(10),
                display_name: "Asghan clone".into(),
                origin: Revision2OriginRef::New {
                    authored_runtime_id: "GoreAsghanClone".into(),
                },
                revision: 0,
                payload: Revision2EntityPayload::NpcDraft(npc),
            },
        ),
        (
            entity_id(11),
            Revision2Entity {
                id: entity_id(11),
                display_name: "Asghan clone script".into(),
                origin: Revision2OriginRef::Generated {
                    generator_id: npc_script.generator_id.clone(),
                    generator_version: npc_script.generator_version,
                    owner: npc_owner,
                },
                revision: 0,
                payload: Revision2EntityPayload::ScriptModule(npc_script),
            },
        ),
        (
            entity_id(20),
            Revision2Entity {
                id: entity_id(20),
                display_name: "Asghan's Trial".into(),
                origin: Revision2OriginRef::New {
                    authored_runtime_id: "GORE_ASGHAN_TRIAL".into(),
                },
                revision: 0,
                payload: Revision2EntityPayload::QuestDraft(quest),
            },
        ),
        (
            entity_id(21),
            Revision2Entity {
                id: entity_id(21),
                display_name: "Asghan's Trial script".into(),
                origin: Revision2OriginRef::Generated {
                    generator_id: quest_script.generator_id.clone(),
                    generator_version: quest_script.generator_version,
                    owner: quest_owner,
                },
                revision: 0,
                payload: Revision2EntityPayload::ScriptModule(quest_script),
            },
        ),
    ]);

    ProjectRevision2 {
        format: FormatV2,
        schema_revision: SchemaRevisionV2,
        project_id,
        revision: 0,
        meta: ProjectMeta {
            name: "Story graph".into(),
            version: "0.1".into(),
            author: "test".into(),
        },
        target,
        authoring_locales: BTreeSet::new(),
        entities,
        asset_store: AssetStoreIndex::default(),
    }
}

fn script_mut(project: &mut ProjectRevision2, id: u8) -> &mut ScriptModule {
    let entity = project.entities.get_mut(&entity_id(id)).unwrap();
    let Revision2EntityPayload::ScriptModule(module) = &mut entity.payload else {
        panic!("expected script module");
    };
    module
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

fn snapshot_bytes(root: &TestRoot, head: &WorkingHead) -> Vec<u8> {
    fs::read(digest_path(
        root.path(),
        "snapshots",
        head.snapshot.sha256,
        ".json",
    ))
    .unwrap()
}

fn snapshot_entity_seal(snapshot: &[u8], id: EntityId) -> ContentSeal {
    let value: serde_json::Value = serde_json::from_slice(snapshot).unwrap();
    serde_json::from_value(value["entities"][id.to_string()].clone()).unwrap()
}

fn sha256(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::from_bytes(Sha256::digest(bytes).into())
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

fn install_candidate_snapshot(root: &TestRoot, bytes: &[u8]) -> Vec<u8> {
    let digest = sha256(bytes);
    let path = digest_path(root.path(), "snapshots", digest, ".json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
    serde_json::to_vec(&WorkingHead {
        store_format: Default::default(),
        snapshot: ContentSeal {
            byte_len: bytes.len() as u64,
            sha256: digest,
        },
    })
    .unwrap()
}

fn revision2_codes(prepared: &gore_authoring::CheckpointPreparation) -> Vec<DiagnosticCode> {
    prepared
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn revision1_document_dispatch_preserves_legacy_bytes_and_api() {
    let root = TestRoot::new("revision1-golden");
    let store = store(&root);
    let project = revision1_project();

    let legacy = store
        .prepare_checkpoint(None, &project, ValidationProfile::Production)
        .unwrap();
    let document = store
        .prepare_document_checkpoint(
            None,
            &ProjectDocument::Revision1(project.clone()),
            ValidationProfile::Production,
        )
        .unwrap();

    assert_eq!(legacy, document);
    assert_eq!(
        String::from_utf8(legacy.head_bytes.clone()).unwrap(),
        "{\"store_format\":1,\"snapshot\":{\"byte_len\":493,\"sha256\":\"31447eb3417ec5201ab28815738b8c7332f9f9e69ca58953ef59ba67e9282898\"}}"
    );
    let old_opened = store
        .open_head_bytes(
            &legacy.head_bytes,
            AssetVerification::Full,
            ValidationProfile::Production,
        )
        .unwrap();
    let document_opened = store
        .open_head_bytes_document(
            &legacy.head_bytes,
            AssetVerification::Full,
            ValidationProfile::Production,
        )
        .unwrap();
    assert_eq!(old_opened.project, project);
    assert_eq!(document_opened.project, ProjectDocument::Revision1(project));
    assert_eq!(old_opened.diagnostics, document_opened.diagnostics);
    assert_eq!(old_opened.blocks_build, document_opened.blocks_build);
}

#[test]
fn revision2_story_checkpoint_reopens_exactly_but_never_claims_full_readiness() {
    let root = TestRoot::new("revision2-roundtrip");
    let store = store(&root);
    let project = story_project();
    let document = ProjectDocument::Revision2(project.clone());

    let experimental = store
        .prepare_document_checkpoint(None, &document, ValidationProfile::Experimental)
        .unwrap();
    assert!(experimental.blocks_build);
    let combined = experimental
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == DiagnosticCode::Revision2CombinedValidationUnavailable
        })
        .unwrap();
    assert_eq!(combined.severity, DiagnosticSeverity::Error);
    assert!(combined.blocks_build);
    assert!(experimental.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::RuntimeUnqualified
            && diagnostic.severity == DiagnosticSeverity::Warning
            && !diagnostic.blocks_build
    }));

    let reopened = store
        .open_head_bytes_document(
            &experimental.head_bytes,
            AssetVerification::Full,
            ValidationProfile::Experimental,
        )
        .unwrap();
    assert_eq!(reopened.project, document);
    assert_eq!(reopened.head, experimental.head);
    assert_eq!(reopened.diagnostics, experimental.diagnostics);
    assert!(reopened.blocks_build);

    let snapshot = snapshot_bytes(&root, &experimental.head);
    for id in [entity_id(10), entity_id(11), entity_id(20), entity_id(21)] {
        let seal = snapshot_entity_seal(&snapshot, id);
        let physical = fs::read(entity_path(root.path(), id, seal.sha256)).unwrap();
        assert_eq!(
            physical,
            serde_json::to_vec(&project.entities[&id]).unwrap()
        );
    }

    publish(&root, &experimental.head_bytes);
    let current = store
        .open_current_document(AssetVerification::Full, ValidationProfile::Experimental)
        .unwrap();
    assert_eq!(current, reopened);

    let production = store
        .prepare_document_checkpoint(
            Some(&experimental.head),
            &ProjectDocument::Revision2(project),
            ValidationProfile::Production,
        )
        .unwrap();
    assert!(production.blocks_build);
    assert!(production.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::RuntimeUnqualified
            && diagnostic.severity == DiagnosticSeverity::Error
            && diagnostic.blocks_build
    }));
    assert!(revision2_codes(&production)
        .contains(&DiagnosticCode::Revision2CombinedValidationUnavailable));
}

#[test]
fn revision2_story_drift_and_ownership_diagnostics_remain_blocking() {
    let drift_root = TestRoot::new("revision2-drift");
    let mut drifted = story_project();
    script_mut(&mut drifted, 11).source.push_str("\n// drift");
    let drift = store(&drift_root)
        .prepare_document_checkpoint(
            None,
            &ProjectDocument::Revision2(drifted),
            ValidationProfile::Experimental,
        )
        .unwrap();
    assert!(drift.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::GeneratedScriptDrift && diagnostic.blocks_build
    }));

    let ownership_root = TestRoot::new("revision2-ownership");
    let mut wrong_owner = story_project();
    let owner = &mut script_mut(&mut wrong_owner, 11).owner;
    owner.id = entity_id(20);
    owner.expected_kind = Revision2EntityKind::QuestDraft;
    let ownership = store(&ownership_root)
        .prepare_document_checkpoint(
            None,
            &ProjectDocument::Revision2(wrong_owner),
            ValidationProfile::Experimental,
        )
        .unwrap();
    assert!(ownership.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::ScriptModuleOwnershipMismatch && diagnostic.blocks_build
    }));
}

#[test]
fn revision2_head_cas_is_identical_to_revision1_contract() {
    let root = TestRoot::new("revision2-cas");
    let store = store(&root);
    let first = store
        .prepare_document_checkpoint(
            None,
            &ProjectDocument::Revision2(story_project()),
            ValidationProfile::Experimental,
        )
        .unwrap();
    publish(&root, &first.head_bytes);

    let mut changed = story_project();
    changed.revision = 1;
    assert!(matches!(
        store.prepare_document_checkpoint(
            None,
            &ProjectDocument::Revision2(changed.clone()),
            ValidationProfile::Experimental,
        ),
        Err(WorkingStoreError::HeadConflict { .. })
    ));
    store
        .prepare_document_checkpoint(
            Some(&first.head),
            &ProjectDocument::Revision2(changed),
            ValidationProfile::Experimental,
        )
        .unwrap();
}

#[test]
fn revision2_entity_preflight_precedes_asset_io_and_has_no_object_side_effects() {
    let missing_digest = Sha256Digest::from_bytes([0x99; 32]);
    let with_missing_asset = |mut project: ProjectRevision2| {
        project.asset_store.assets.insert(
            missing_digest,
            AssetMeta {
                byte_len: 1,
                media_type: "application/octet-stream".into(),
            },
        );
        project
    };
    let assert_no_objects = |root: &TestRoot| {
        assert!(!root.path().join("entities").exists());
        assert!(!root.path().join("snapshots").exists());
    };

    let key_root = TestRoot::new("revision2-preflight-key");
    let mut wrong_key = with_missing_asset(story_project());
    let entity = wrong_key.entities.remove(&entity_id(10)).unwrap();
    wrong_key.entities.insert(entity_id(9), entity);
    assert!(matches!(
        store(&key_root).prepare_document_checkpoint(
            None,
            &ProjectDocument::Revision2(wrong_key),
            ValidationProfile::Experimental,
        ),
        Err(WorkingStoreError::Invariant(message)) if message.contains("entity map key")
    ));
    assert_no_objects(&key_root);

    let size_root = TestRoot::new("revision2-preflight-size");
    let size_store = WorkingProjectStore::at(
        size_root.path(),
        WorkingStoreLimits {
            max_entity_bytes: 64,
            ..WorkingStoreLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        size_store.prepare_document_checkpoint(
            None,
            &ProjectDocument::Revision2(with_missing_asset(story_project())),
            ValidationProfile::Experimental,
        ),
        Err(WorkingStoreError::LimitExceeded {
            kind: "entity bytes",
            ..
        })
    ));
    assert_no_objects(&size_root);

    let aggregate_project = with_missing_asset(story_project());
    let aggregate_bytes: u64 = aggregate_project
        .entities
        .values()
        .map(|entity| serde_json::to_vec(entity).unwrap().len() as u64)
        .sum();
    let aggregate_root = TestRoot::new("revision2-preflight-aggregate");
    let aggregate_store = WorkingProjectStore::at(
        aggregate_root.path(),
        WorkingStoreLimits {
            max_referenced_entity_bytes: aggregate_bytes - 1,
            ..WorkingStoreLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        aggregate_store.prepare_document_checkpoint(
            None,
            &ProjectDocument::Revision2(aggregate_project),
            ValidationProfile::Experimental,
        ),
        Err(WorkingStoreError::LimitExceeded {
            kind: "aggregate referenced entity bytes",
            ..
        })
    ));
    assert_no_objects(&aggregate_root);
}

#[test]
fn revision2_missing_corrupt_noncanonical_and_duplicate_shards_fail_closed() {
    let missing_root = TestRoot::new("revision2-missing-shard");
    let missing_store = store(&missing_root);
    let missing = missing_store
        .prepare_document_checkpoint(
            None,
            &ProjectDocument::Revision2(story_project()),
            ValidationProfile::Experimental,
        )
        .unwrap();
    let snapshot = snapshot_bytes(&missing_root, &missing.head);
    let id = entity_id(10);
    let entity_seal = snapshot_entity_seal(&snapshot, id);
    fs::remove_file(entity_path(missing_root.path(), id, entity_seal.sha256)).unwrap();
    assert!(matches!(
        missing_store.open_head_bytes_document(
            &missing.head_bytes,
            AssetVerification::Full,
            ValidationProfile::Experimental,
        ),
        Err(WorkingStoreError::MissingObject(_))
    ));

    let corrupt_root = TestRoot::new("revision2-corrupt-shard");
    let corrupt_store = store(&corrupt_root);
    let corrupt = corrupt_store
        .prepare_document_checkpoint(
            None,
            &ProjectDocument::Revision2(story_project()),
            ValidationProfile::Experimental,
        )
        .unwrap();
    let snapshot = snapshot_bytes(&corrupt_root, &corrupt.head);
    let entity_seal = snapshot_entity_seal(&snapshot, id);
    fs::write(
        entity_path(corrupt_root.path(), id, entity_seal.sha256),
        vec![b'x'; entity_seal.byte_len as usize],
    )
    .unwrap();
    assert!(matches!(
        corrupt_store.open_head_bytes_document(
            &corrupt.head_bytes,
            AssetVerification::Full,
            ValidationProfile::Experimental,
        ),
        Err(WorkingStoreError::SealMismatch { .. })
    ));

    for (label, mutate, expected_duplicate) in [
        (
            "noncanonical",
            Box::new(|bytes: Vec<u8>| {
                let mut changed = bytes;
                changed.push(b' ');
                changed
            }) as Box<dyn Fn(Vec<u8>) -> Vec<u8>>,
            false,
        ),
        (
            "duplicate",
            Box::new(|bytes: Vec<u8>| {
                let text = String::from_utf8(bytes).unwrap();
                text.replacen(
                    "\"display_name\":\"Asghan clone\"",
                    "\"display_name\":\"Asghan clone\",\"display_name\":\"Asghan clone\"",
                    1,
                )
                .into_bytes()
            }),
            true,
        ),
    ] {
        let root = TestRoot::new(label);
        let store = store(&root);
        let prepared = store
            .prepare_document_checkpoint(
                None,
                &ProjectDocument::Revision2(story_project()),
                ValidationProfile::Experimental,
            )
            .unwrap();
        let snapshot = snapshot_bytes(&root, &prepared.head);
        let old_seal = snapshot_entity_seal(&snapshot, id);
        let old_bytes = fs::read(entity_path(root.path(), id, old_seal.sha256)).unwrap();
        let changed = mutate(old_bytes);
        let new_seal = ContentSeal {
            byte_len: changed.len() as u64,
            sha256: sha256(&changed),
        };
        let path = entity_path(root.path(), id, new_seal.sha256);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, changed).unwrap();
        let candidate_snapshot = replace_snapshot_entity_seal(&snapshot, id, &old_seal, &new_seal);
        let candidate_head = install_candidate_snapshot(&root, &candidate_snapshot);
        let error = store
            .open_head_bytes_document(
                &candidate_head,
                AssetVerification::Full,
                ValidationProfile::Experimental,
            )
            .unwrap_err();
        if expected_duplicate {
            assert!(matches!(
                error,
                WorkingStoreError::InvalidJson {
                    kind: "revision-2 entity",
                    ..
                }
            ));
        } else {
            assert!(matches!(
                error,
                WorkingStoreError::NonCanonicalJson {
                    kind: "revision-2 entity"
                }
            ));
        }
    }
}

#[test]
fn revision2_snapshot_parser_rejects_unknown_duplicate_and_noncanonical_json() {
    let root = TestRoot::new("revision2-strict-snapshot");
    let store = store(&root);
    let prepared = store
        .prepare_document_checkpoint(
            None,
            &ProjectDocument::Revision2(story_project()),
            ValidationProfile::Experimental,
        )
        .unwrap();
    let snapshot = snapshot_bytes(&root, &prepared.head);
    let text = String::from_utf8(snapshot.clone()).unwrap();

    let mut unknown = text.as_bytes()[..text.len() - 1].to_vec();
    unknown.extend_from_slice(b",\"unexpected\":0}");
    let duplicate = text
        .replacen(
            "\"schema_revision\":2",
            "\"schema_revision\":2,\"schema_revision\":2",
            1,
        )
        .into_bytes();
    let mut noncanonical = snapshot;
    noncanonical.push(b' ');

    for (candidate, expected) in [
        (unknown, "unknown"),
        (duplicate, "duplicate"),
        (noncanonical, "noncanonical"),
    ] {
        let head = install_candidate_snapshot(&root, &candidate);
        let error = store
            .open_head_bytes_document(
                &head,
                AssetVerification::Full,
                ValidationProfile::Experimental,
            )
            .unwrap_err();
        match expected {
            "unknown" => assert!(matches!(
                error,
                WorkingStoreError::InvalidJson {
                    kind: "revision-2 snapshot",
                    ..
                }
            )),
            "duplicate" => assert!(matches!(
                error,
                WorkingStoreError::InvalidJson {
                    kind: "snapshot revision probe",
                    ..
                }
            )),
            "noncanonical" => assert!(matches!(
                error,
                WorkingStoreError::NonCanonicalJson {
                    kind: "revision-2 snapshot"
                }
            )),
            _ => unreachable!(),
        }
    }
}

#[test]
fn revision2_structural_production_open_still_has_the_combined_validator_gate() {
    let root = TestRoot::new("revision2-structural-production-gate");
    let store = store(&root);
    let project = story_project();
    let prepared = store
        .prepare_document_checkpoint(
            None,
            &ProjectDocument::Revision2(project.clone()),
            ValidationProfile::Experimental,
        )
        .unwrap();

    let opened = store
        .open_head_bytes_document(
            &prepared.head_bytes,
            AssetVerification::Structural,
            ValidationProfile::Production,
        )
        .unwrap();
    assert_eq!(opened.project, ProjectDocument::Revision2(project));
    assert!(opened.blocks_build);
    let combined = opened
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == DiagnosticCode::Revision2CombinedValidationUnavailable
        })
        .unwrap();
    assert_eq!(combined.severity, DiagnosticSeverity::Error);
    assert!(combined.blocks_build);
}

#[test]
fn revision2_entity_count_size_and_aggregate_limits_apply_on_prepare_and_open() {
    let project = story_project();
    let count_root = TestRoot::new("revision2-count-limit");
    let count_store = WorkingProjectStore::at(
        count_root.path(),
        WorkingStoreLimits {
            max_entities: 3,
            ..WorkingStoreLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        count_store.prepare_document_checkpoint(
            None,
            &ProjectDocument::Revision2(project.clone()),
            ValidationProfile::Experimental,
        ),
        Err(WorkingStoreError::LimitExceeded {
            kind: "entity count",
            ..
        })
    ));

    let size_root = TestRoot::new("revision2-size-limit");
    let size_store = WorkingProjectStore::at(
        size_root.path(),
        WorkingStoreLimits {
            max_entity_bytes: 64,
            ..WorkingStoreLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        size_store.prepare_document_checkpoint(
            None,
            &ProjectDocument::Revision2(project.clone()),
            ValidationProfile::Experimental,
        ),
        Err(WorkingStoreError::LimitExceeded {
            kind: "entity bytes",
            ..
        })
    ));

    let aggregate_bytes: u64 = project
        .entities
        .values()
        .map(|entity| serde_json::to_vec(entity).unwrap().len() as u64)
        .sum();
    let aggregate_root = TestRoot::new("revision2-aggregate-limit");
    let aggregate_store = WorkingProjectStore::at(
        aggregate_root.path(),
        WorkingStoreLimits {
            max_referenced_entity_bytes: aggregate_bytes - 1,
            ..WorkingStoreLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        aggregate_store.prepare_document_checkpoint(
            None,
            &ProjectDocument::Revision2(project.clone()),
            ValidationProfile::Experimental,
        ),
        Err(WorkingStoreError::LimitExceeded {
            kind: "aggregate referenced entity bytes",
            ..
        })
    ));

    let reopen_root = TestRoot::new("revision2-aggregate-reopen");
    let prepared = store(&reopen_root)
        .prepare_document_checkpoint(
            None,
            &ProjectDocument::Revision2(project),
            ValidationProfile::Experimental,
        )
        .unwrap();
    let constrained = WorkingProjectStore::at(
        reopen_root.path(),
        WorkingStoreLimits {
            max_referenced_entity_bytes: aggregate_bytes - 1,
            ..WorkingStoreLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        constrained.open_head_bytes_document(
            &prepared.head_bytes,
            AssetVerification::Full,
            ValidationProfile::Experimental,
        ),
        Err(WorkingStoreError::LimitExceeded {
            kind: "aggregate referenced entity bytes",
            ..
        })
    ));
}

#[test]
fn revision2_voice_takes_receive_full_ogg_verification_or_fail_closed() {
    let root = TestRoot::new("revision2-voice-full");
    let store = store(&root);
    let source = root.path().join("voice.ogg");
    fs::write(
        &source,
        include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg"),
    )
    .unwrap();
    let imported = store
        .import_ogg(&source, "takes/revision2.ogg", None)
        .unwrap();

    let mut project = story_project();
    let take_id = entity_id(30);
    project.entities.insert(
        take_id,
        Revision2Entity {
            id: take_id,
            display_name: "Revision-2 voice".into(),
            origin: Revision2OriginRef::Imported {
                importer: "working_store_import_ogg_v1".into(),
                source_seal: ContentSeal {
                    byte_len: imported.asset.byte_len,
                    sha256: imported.asset.sha256,
                },
                external_identity: None,
            },
            revision: 0,
            payload: Revision2EntityPayload::VoiceTake(Revision2VoiceTake {
                locale: locale("de"),
                asset: imported.asset.clone(),
                ogg: Revision2OggMetadata {
                    codec: match imported.ogg.codec {
                        gore_authoring::OggCodec::Vorbis => Revision2OggCodec::Vorbis,
                        gore_authoring::OggCodec::Opus => Revision2OggCodec::Opus,
                    },
                    channels: imported.ogg.channels,
                    sample_rate: imported.ogg.sample_rate,
                    pages: imported.ogg.pages,
                    logical_streams: imported.ogg.logical_streams,
                },
                status: Revision2VoiceTakeStatus::Draft,
            }),
        },
    );
    project.asset_store.assets.insert(
        imported.asset.sha256,
        AssetMeta {
            byte_len: imported.asset.byte_len,
            media_type: "audio/ogg".into(),
        },
    );
    store
        .prepare_document_checkpoint(
            None,
            &ProjectDocument::Revision2(project.clone()),
            ValidationProfile::Experimental,
        )
        .unwrap();

    let Revision2EntityPayload::VoiceTake(take) =
        &mut project.entities.get_mut(&take_id).unwrap().payload
    else {
        unreachable!();
    };
    take.ogg.sample_rate += 1;
    assert!(matches!(
        store.prepare_document_checkpoint(
            None,
            &ProjectDocument::Revision2(project.clone()),
            ValidationProfile::Experimental,
        ),
        Err(WorkingStoreError::OggMetadataMismatch { entity, .. }) if entity == take_id
    ));

    project.asset_store.assets.clear();
    assert!(matches!(
        store.prepare_document_checkpoint(
            None,
            &ProjectDocument::Revision2(project),
            ValidationProfile::Experimental,
        ),
        Err(WorkingStoreError::Invariant(message)) if message.contains("absent from asset_store")
    ));
}
