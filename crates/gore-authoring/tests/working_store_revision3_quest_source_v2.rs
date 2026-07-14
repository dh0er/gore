use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use gore_authoring::{
    AssetMeta, AssetStoreIndex, AssetVerification, ContentSeal, EntityId, FormatV2,
    GameGenerationAnchor, ProjectId, ProjectMeta, ProjectRevision3, QuestCollisionArtifactRef,
    QuestCollisionCatalogInput, Revision3Entity, Revision3EntityKind, Revision3EntityPayload,
    Revision3OriginRef, Revision3QuestDraft, Revision3QuestDraftInput, Revision3QuestGiverInput,
    Revision3QuestParentInput, Revision3SnapshotManifest, Revision3TypedRef, SchemaRevisionV3,
    Sha256Digest, WorkingProjectStore, WorkingStoreLimits, QUEST_COLLISION_ARTIFACT_MEDIA_TYPE,
    QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2, QUEST_COLLISION_CATALOG_LAYER,
    QUEST_COLLISION_CATALOG_LAYER_V2, REVISION3_QUEST_GENERATOR_ID,
    REVISION3_QUEST_GENERATOR_VERSION,
};
use sha2::{Digest as _, Sha256};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);

impl TestRoot {
    fn new(label: &str) -> Self {
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "gore-authoring-current-quest-source-{label}-{}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn id(value: u8) -> EntityId {
    EntityId::from_bytes([value; 16])
}

fn seal(value: u8, byte_len: u64) -> ContentSeal {
    ContentSeal {
        byte_len,
        sha256: Sha256Digest::from_bytes([value; 32]),
    }
}

fn target() -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: seal(1, 170_000_000),
    }
}

fn empty_project() -> ProjectRevision3 {
    ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id: ProjectId::from_bytes([3; 16]),
        revision: 7,
        meta: ProjectMeta {
            name: "Current Quest source".into(),
            version: "0.1.0".into(),
            author: "tests".into(),
        },
        target: target(),
        authoring_locales: BTreeSet::new(),
        entities: BTreeMap::new(),
        asset_store: AssetStoreIndex::default(),
    }
}

fn artifact_ref(raw: ContentSeal, basis: ContentSeal, layer: &str) -> QuestCollisionArtifactRef {
    QuestCollisionArtifactRef {
        generation: target(),
        catalog_layer: layer.into(),
        source_seal: ContentSeal {
            byte_len: raw.byte_len,
            sha256: Sha256Digest::from_bytes([9; 32]),
        },
        artifact: raw,
        basis_snapshot: basis,
    }
}

fn add_quest(
    project: &mut ProjectRevision3,
    quest_value: u8,
    module_value: u8,
    suffix: &str,
    artifact: QuestCollisionArtifactRef,
) {
    let quest_id = id(quest_value);
    let module_id = id(module_value);
    let quest = Revision3QuestDraft {
        generator_id: REVISION3_QUEST_GENERATOR_ID.into(),
        generator_version: REVISION3_QUEST_GENERATOR_VERSION,
        input: Revision3QuestDraftInput {
            target: target(),
            quest_id,
            module_namespace: format!("GoreMods.Quests.{suffix}"),
            technical_id: format!("GORE_{}", suffix.to_ascii_uppercase()),
            text_helper: format!("GoreQuestText{suffix}"),
            parent_quest: Revision3QuestParentInput {
                generation: target(),
                source_seal: seal(2, 2_000),
                catalog_layer: "base-game.g1r.quests".into(),
                canonical_selector: "CatalogQuest_Parent".into(),
                runtime_class: "UQuest_Parent".into(),
            },
            giver: Revision3QuestGiverInput {
                generation: target(),
                source_seal: seal(3, 3_000),
                catalog_layer: "base-game.g1r.characters".into(),
                canonical_selector: "CatalogCharacter_Asghan".into(),
                runtime_unique_name: "OM_GRD_Asghan_263".into(),
            },
            title: format!("Quest {suffix}"),
            description: "Exact current-project source evidence".into(),
            objective_title: "Regenerate without historical authority".into(),
            additional_objective_titles: Vec::new(),
            transition_plan: None,
            collision_catalog: artifact,
        },
        script_module: Revision3TypedRef::new(
            project.project_id,
            module_id,
            Revision3EntityKind::ScriptModule,
        ),
    };
    let collision = QuestCollisionCatalogInput {
        generation: quest.input.collision_catalog.generation.clone(),
        source_seal: quest.input.collision_catalog.source_seal.clone(),
        catalog_layer: quest.input.collision_catalog.catalog_layer.clone(),
        modules: BTreeSet::new(),
        relative_paths: BTreeSet::new(),
        symbols: BTreeSet::new(),
    };
    let module = gore_authoring::regenerate_revision3_quest_module_v2(&quest, collision).unwrap();
    let owner = Revision3TypedRef::new(
        project.project_id,
        quest_id,
        Revision3EntityKind::QuestDraft,
    );
    project.entities.insert(
        quest_id,
        Revision3Entity {
            id: quest_id,
            display_name: format!("Quest {suffix}"),
            origin: Revision3OriginRef::New {
                authored_runtime_id: quest.input.technical_id.clone(),
            },
            revision: 0,
            payload: Revision3EntityPayload::QuestDraft(quest),
        },
    );
    project.entities.insert(
        module_id,
        Revision3Entity {
            id: module_id,
            display_name: format!("Quest {suffix} module"),
            origin: Revision3OriginRef::Generated {
                generator_id: REVISION3_QUEST_GENERATOR_ID.into(),
                generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                owner,
            },
            revision: 0,
            payload: Revision3EntityPayload::ScriptModule(module),
        },
    );
}

fn object_path(root: &Path, directory: &str, digest: Sha256Digest, extension: &str) -> PathBuf {
    let hex = digest.to_string();
    root.join(directory)
        .join("sha256")
        .join(&hex[..2])
        .join(format!("{}{}", &hex[2..], extension))
}

fn entity_path(root: &Path, entity: EntityId, digest: Sha256Digest) -> PathBuf {
    let id = entity.to_string();
    root.join("entities")
        .join(&id[..2])
        .join(&id[2..])
        .join(format!("{digest}.json"))
}

#[test]
fn exact_current_source_ignores_deleted_historical_artifact_and_basis() {
    let root = TestRoot::new("deleted-history");
    let store = WorkingProjectStore::at(&root.0, WorkingStoreLimits::default()).unwrap();
    let basis_project = empty_project();
    let basis = store
        .prepare_revision3_checkpoint(None, &basis_project)
        .unwrap();
    fs::write(root.0.join("gore-project.json"), &basis.head_bytes).unwrap();
    let imported = store
        .import_quest_collision_artifact_v1(b"{}", Some(&basis.head))
        .unwrap();

    let mut current = basis_project;
    current.revision += 1;
    current
        .asset_store
        .assets
        .insert(imported.artifact.sha256, imported.asset_meta.clone());
    let orphan_history_bytes = b"orphan collision artifact history";
    let orphan_history_digest =
        Sha256Digest::from_bytes(Sha256::digest(orphan_history_bytes).into());
    let orphan_history_path = object_path(&root.0, "assets", orphan_history_digest, "");
    fs::create_dir_all(orphan_history_path.parent().unwrap()).unwrap();
    fs::write(&orphan_history_path, orphan_history_bytes).unwrap();
    current.asset_store.assets.insert(
        orphan_history_digest,
        AssetMeta {
            byte_len: orphan_history_bytes.len() as u64,
            media_type: QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2.into(),
        },
    );
    let reference = artifact_ref(
        imported.artifact.clone(),
        basis.head.snapshot.clone(),
        QUEST_COLLISION_CATALOG_LAYER,
    );
    add_quest(&mut current, 10, 11, "First", reference.clone());
    let one_quest = store
        .prepare_revision3_checkpoint(Some(&basis.head), &current)
        .unwrap();
    fs::write(root.0.join("gore-project.json"), &one_quest.head_bytes).unwrap();
    assert_eq!(
        store
            .prepare_current_revision3_quest_collision_source_v2(&one_quest.head)
            .unwrap()
            .prior_quest_count(),
        1
    );

    current.revision += 1;
    add_quest(&mut current, 20, 21, "Second", reference);
    let prepared = store
        .prepare_revision3_checkpoint(Some(&one_quest.head), &current)
        .unwrap();
    fs::write(root.0.join("gore-project.json"), &prepared.head_bytes).unwrap();

    let source = store
        .prepare_current_revision3_quest_collision_source_v2(&prepared.head)
        .unwrap();
    assert_eq!(source.prior_quest_count(), 2);
    assert!(source.nonquest_basis().project().entities.is_empty());
    assert!(source
        .nonquest_basis()
        .project()
        .asset_store
        .assets
        .is_empty());
    assert_eq!(source.nonquest_basis().project().revision, 9);
    assert_eq!(source.current_snapshot(), &prepared.head.snapshot);
    assert_eq!(
        source.current_project().byte_len as usize,
        current.to_canonical_json().unwrap().len()
    );

    fs::remove_file(object_path(&root.0, "assets", imported.artifact.sha256, "")).unwrap();
    fs::remove_file(&orphan_history_path).unwrap();
    fs::remove_file(object_path(
        &root.0,
        "snapshots",
        basis.head.snapshot.sha256,
        ".json",
    ))
    .unwrap();
    assert_eq!(
        store
            .prepare_current_revision3_quest_collision_source_v2(&prepared.head)
            .unwrap()
            .prior_quest_count(),
        2
    );

    // Corrupt historical objects are equally irrelevant: the current-project path must not even
    // inspect their length or hash.
    fs::write(
        object_path(&root.0, "assets", imported.artifact.sha256, ""),
        b"corrupt historical artifact",
    )
    .unwrap();
    fs::write(&orphan_history_path, b"corrupt orphan historical artifact").unwrap();
    fs::write(
        object_path(&root.0, "snapshots", basis.head.snapshot.sha256, ".json"),
        b"corrupt historical basis",
    )
    .unwrap();
    assert_eq!(
        store
            .prepare_current_revision3_quest_collision_source_v2(&prepared.head)
            .unwrap()
            .prior_quest_count(),
        2
    );

    let current_snapshot_path =
        object_path(&root.0, "snapshots", prepared.head.snapshot.sha256, ".json");
    let current_snapshot_bytes = fs::read(&current_snapshot_path).unwrap();
    let current_manifest: Revision3SnapshotManifest =
        serde_json::from_slice(&current_snapshot_bytes).unwrap();
    let quest_entity_path = entity_path(&root.0, id(10), current_manifest.entities[&id(10)].sha256);
    let quest_entity_bytes = fs::read(&quest_entity_path).unwrap();
    fs::write(&quest_entity_path, vec![b'!'; quest_entity_bytes.len()]).unwrap();
    assert!(store
        .prepare_current_revision3_quest_collision_source_v2(&prepared.head)
        .is_err());
    fs::write(&quest_entity_path, quest_entity_bytes).unwrap();
    assert!(store
        .prepare_current_revision3_quest_collision_source_v2(&prepared.head)
        .is_ok());

    fs::write(
        current_snapshot_path,
        vec![b'!'; current_snapshot_bytes.len()],
    )
    .unwrap();
    assert!(store
        .prepare_current_revision3_quest_collision_source_v2(&prepared.head)
        .is_err());
}

#[test]
fn v1_and_v2_layer_media_pairs_are_exact() {
    let raw = seal(7, 2);
    let basis = seal(8, 200);
    for (layer, media) in [
        (
            QUEST_COLLISION_CATALOG_LAYER,
            QUEST_COLLISION_ARTIFACT_MEDIA_TYPE,
        ),
        (
            QUEST_COLLISION_CATALOG_LAYER_V2,
            QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2,
        ),
    ] {
        let mut project = empty_project();
        project.asset_store.assets.insert(
            raw.sha256,
            AssetMeta {
                byte_len: raw.byte_len,
                media_type: media.into(),
            },
        );
        add_quest(
            &mut project,
            10,
            11,
            "Pairing",
            artifact_ref(raw.clone(), basis.clone(), layer),
        );
        assert!(project.validate_closed_model().is_ok());

        project
            .asset_store
            .assets
            .get_mut(&raw.sha256)
            .unwrap()
            .media_type = if media == QUEST_COLLISION_ARTIFACT_MEDIA_TYPE {
            QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2.into()
        } else {
            QUEST_COLLISION_ARTIFACT_MEDIA_TYPE.into()
        };
        assert!(project.validate_closed_model().is_err());
    }
}

#[test]
fn persisted_module_and_runtime_case_drift_fail_closed() {
    let root = TestRoot::new("runtime-collision");
    let store = WorkingProjectStore::at(&root.0, WorkingStoreLimits::default()).unwrap();
    let basis_project = empty_project();
    let basis = store
        .prepare_revision3_checkpoint(None, &basis_project)
        .unwrap();
    fs::write(root.0.join("gore-project.json"), &basis.head_bytes).unwrap();
    let imported = store
        .import_quest_collision_artifact_v1(b"{}", Some(&basis.head))
        .unwrap();

    let mut project = basis_project;
    project.revision += 1;
    project
        .asset_store
        .assets
        .insert(imported.artifact.sha256, imported.asset_meta);
    let reference = artifact_ref(
        imported.artifact,
        basis.head.snapshot.clone(),
        QUEST_COLLISION_CATALOG_LAYER,
    );
    add_quest(&mut project, 10, 11, "Same", reference.clone());
    add_quest(&mut project, 20, 21, "Other", reference);
    let Revision3EntityPayload::QuestDraft(mut second) =
        project.entities.get(&id(20)).unwrap().payload.clone()
    else {
        panic!("expected Quest")
    };
    second.input.technical_id = "GORE_SAME".into();
    project.entities.get_mut(&id(20)).unwrap().origin = Revision3OriginRef::New {
        authored_runtime_id: "GORE_SAME".into(),
    };
    let collision = QuestCollisionCatalogInput {
        generation: second.input.collision_catalog.generation.clone(),
        source_seal: second.input.collision_catalog.source_seal.clone(),
        catalog_layer: second.input.collision_catalog.catalog_layer.clone(),
        modules: BTreeSet::new(),
        relative_paths: BTreeSet::new(),
        symbols: BTreeSet::new(),
    };
    let regenerated =
        gore_authoring::regenerate_revision3_quest_module_v2(&second, collision).unwrap();
    project.entities.get_mut(&id(20)).unwrap().payload = Revision3EntityPayload::QuestDraft(second);
    project.entities.get_mut(&id(21)).unwrap().payload =
        Revision3EntityPayload::ScriptModule(regenerated);

    let current = store
        .prepare_revision3_checkpoint(Some(&basis.head), &project)
        .unwrap();
    fs::write(root.0.join("gore-project.json"), &current.head_bytes).unwrap();
    assert!(matches!(
        store.prepare_current_revision3_quest_collision_source_v2(&current.head),
        Err(gore_authoring::Revision3QuestCollisionSourceErrorV2::DuplicateRuntimeId { .. })
    ));
}

#[test]
fn a_corrupt_nonquest_asset_is_not_skipped() {
    let root = TestRoot::new("nonquest-asset");
    let store = WorkingProjectStore::at(&root.0, WorkingStoreLimits::default()).unwrap();
    let bytes = b"ordinary physical asset";
    let digest = Sha256Digest::from_bytes(Sha256::digest(bytes).into());
    let path = object_path(&root.0, "assets", digest, "");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, bytes).unwrap();

    let mut project = empty_project();
    project.asset_store.assets.insert(
        digest,
        AssetMeta {
            byte_len: bytes.len() as u64,
            media_type: "application/octet-stream".into(),
        },
    );
    let prepared = store.prepare_revision3_checkpoint(None, &project).unwrap();
    fs::write(root.0.join("gore-project.json"), &prepared.head_bytes).unwrap();
    assert!(store
        .prepare_current_revision3_quest_collision_source_v2(&prepared.head)
        .is_ok());

    fs::write(path, vec![b'!'; bytes.len()]).unwrap();
    assert!(store
        .prepare_current_revision3_quest_collision_source_v2(&prepared.head)
        .is_err());
    assert!(store
        .open_current_revision3(AssetVerification::Structural)
        .is_ok());
}
