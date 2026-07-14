use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use gore_authoring::{
    AssetMeta, AssetStoreIndex, AssetVerification, ContentSeal, EntityId, FormatV2,
    GameGenerationAnchor, ProjectDocument, ProjectId, ProjectMeta, ProjectRevision3, ProjectV2,
    Revision3Entity, Revision3EntityKind, Revision3EntityPayload, Revision3OriginRef,
    Revision3QuestDraft, Revision3QuestDraftInput, Revision3QuestGiverInput,
    Revision3QuestParentInput, Revision3ScriptModule, Revision3TypedRef, SchemaRevisionV1,
    SchemaRevisionV3, ScriptModuleStatus, Sha256Digest, ValidationProfile, WorkingProjectStore,
    WorkingStoreError, WorkingStoreLimits, MAX_QUEST_COLLISION_ARTIFACT_BYTES,
    MAX_REVISION3_ENTITY_JSON_BYTES, QUEST_COLLISION_ARTIFACT_MEDIA_TYPE,
    QUEST_COLLISION_CATALOG_LAYER, REVISION3_QUEST_GENERATOR_ID, REVISION3_QUEST_GENERATOR_VERSION,
};
use serde::Serialize;
use sha2::{Digest as _, Sha256};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);

impl TestRoot {
    fn new(label: &str) -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "gore-authoring-r3-store-{label}-{}-{sequence}",
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

fn store(root: &TestRoot) -> WorkingProjectStore {
    WorkingProjectStore::at(root.path(), WorkingStoreLimits::default()).unwrap()
}

fn project_id(value: u8) -> ProjectId {
    ProjectId::from_bytes([value; 16])
}

fn entity_id(value: u8) -> EntityId {
    EntityId::from_bytes([value; 16])
}

fn seal(value: u8, byte_len: u64) -> ContentSeal {
    ContentSeal {
        byte_len,
        sha256: Sha256Digest::from_bytes([value; 32]),
    }
}

fn raw_seal(bytes: &[u8]) -> ContentSeal {
    ContentSeal {
        byte_len: bytes.len() as u64,
        sha256: Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
    }
}

fn target() -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: seal(1, 171_698_176),
    }
}

fn empty_revision3() -> ProjectRevision3 {
    ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id: project_id(3),
        revision: 7,
        meta: ProjectMeta {
            name: "Revision 3 store".into(),
            version: "0.1.0".into(),
            author: "tests".into(),
        },
        target: target(),
        authoring_locales: BTreeSet::new(),
        entities: BTreeMap::new(),
        asset_store: AssetStoreIndex::default(),
    }
}

fn empty_revision1() -> ProjectV2 {
    ProjectV2 {
        format: FormatV2,
        schema_revision: SchemaRevisionV1,
        project_id: project_id(1),
        revision: 1,
        meta: ProjectMeta {
            name: "Revision 1 basis impostor".into(),
            version: "0.1.0".into(),
            author: "tests".into(),
        },
        target: target(),
        authoring_locales: BTreeSet::new(),
        entities: BTreeMap::new(),
        asset_store: AssetStoreIndex::default(),
    }
}

#[derive(Serialize)]
struct OpaqueCanonicalArtifact<'a> {
    padding: &'a str,
}

fn opaque_canonical_artifact_bytes(byte_len: usize) -> Vec<u8> {
    let overhead = serde_json::to_vec(&OpaqueCanonicalArtifact { padding: "" })
        .unwrap()
        .len();
    assert!(byte_len > overhead);
    let padding = "x".repeat(byte_len - overhead);
    let bytes = serde_json::to_vec(&OpaqueCanonicalArtifact { padding: &padding }).unwrap();
    assert_eq!(bytes.len(), byte_len);
    bytes
}

fn quest_project(
    basis_snapshot: ContentSeal,
    artifact: ContentSeal,
    asset_meta: AssetMeta,
) -> ProjectRevision3 {
    let mut project = empty_revision3();
    project.revision += 1;
    project
        .asset_store
        .assets
        .insert(artifact.sha256, asset_meta);
    let quest_id = entity_id(10);
    let module_id = entity_id(11);
    let quest = Revision3QuestDraft {
        generator_id: REVISION3_QUEST_GENERATOR_ID.into(),
        generator_version: REVISION3_QUEST_GENERATOR_VERSION,
        input: Revision3QuestDraftInput {
            target: target(),
            quest_id,
            module_namespace: "GoreMods.Quests.StoreTrial".into(),
            technical_id: "GORE_STORE_TRIAL".into(),
            text_helper: "GoreQuestText".into(),
            parent_quest: Revision3QuestParentInput {
                generation: target(),
                source_seal: seal(2, 20_000),
                catalog_layer: "base-game.g1r.quests".into(),
                canonical_selector: "CatalogQuest_Parent".into(),
                runtime_class: "UQuest_Parent".into(),
            },
            giver: Revision3QuestGiverInput {
                generation: target(),
                source_seal: seal(3, 30_000),
                catalog_layer: "base-game.g1r.characters".into(),
                canonical_selector: "CatalogCharacter_Asghan".into(),
                runtime_unique_name: "OM_GRD_Asghan_263".into(),
            },
            title: "Store Trial".into(),
            description: "Keep the collision inventory outside the entity shard.".into(),
            objective_title: "Reopen the basis snapshot".into(),
            additional_objective_titles: Vec::new(),
            collision_catalog: gore_authoring::QuestCollisionArtifactRef {
                generation: target(),
                catalog_layer: QUEST_COLLISION_CATALOG_LAYER.into(),
                artifact: artifact.clone(),
                source_seal: seal(5, artifact.byte_len),
                basis_snapshot,
            },
        },
        script_module: Revision3TypedRef::new(
            project.project_id,
            module_id,
            Revision3EntityKind::ScriptModule,
        ),
    };
    let source = "// persisted revision-3 Quest module\n".to_owned();
    let owner = Revision3TypedRef::new(
        project.project_id,
        quest_id,
        Revision3EntityKind::QuestDraft,
    );
    let module = Revision3ScriptModule {
        generator_id: REVISION3_QUEST_GENERATOR_ID.into(),
        generator_version: REVISION3_QUEST_GENERATOR_VERSION,
        owner: owner.clone(),
        module_namespace: quest.input.module_namespace.clone(),
        module_relative_path: "GoreMods/Quests/StoreTrial.as".into(),
        source_sha256: Sha256Digest::from_bytes(Sha256::digest(source.as_bytes()).into()),
        source,
        input_fingerprint: Sha256Digest::from_bytes([7; 32]),
        status: ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED,
    };
    project.entities.insert(
        quest_id,
        Revision3Entity {
            id: quest_id,
            display_name: "Store Trial Quest".into(),
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
            display_name: "Store Trial source".into(),
            origin: Revision3OriginRef::Generated {
                generator_id: REVISION3_QUEST_GENERATOR_ID.into(),
                generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                owner,
            },
            revision: 0,
            payload: Revision3EntityPayload::ScriptModule(module),
        },
    );
    project
}

fn publish(root: &TestRoot, head_bytes: &[u8]) {
    fs::write(root.path().join("gore-project.json"), head_bytes).unwrap();
}

fn asset_path(root: &Path, digest: Sha256Digest) -> PathBuf {
    let hex = digest.to_string();
    root.join("assets")
        .join("sha256")
        .join(&hex[..2])
        .join(&hex[2..])
}

fn snapshot_path(root: &Path, digest: Sha256Digest) -> PathBuf {
    let hex = digest.to_string();
    root.join("snapshots")
        .join("sha256")
        .join(&hex[..2])
        .join(format!("{}.json", &hex[2..]))
}

#[test]
fn external_artifact_roundtrip_is_deterministic_and_old_basis_survives_head_advance() {
    const REAL_GOLDEN_ARTIFACT_BYTES: usize = 3_517_569;

    let root = TestRoot::new("roundtrip-basis");
    let store = store(&root);
    let basis_project = empty_revision3();
    assert!(matches!(
        store.prepare_document_checkpoint(
            None,
            &ProjectDocument::Revision3(basis_project.clone()),
            ValidationProfile::Experimental,
        ),
        Err(WorkingStoreError::Invariant(message))
            if message.contains("do not authorize schema revision 3")
    ));
    let basis_first = store
        .prepare_revision3_checkpoint(None, &basis_project)
        .unwrap();
    let basis_second = store
        .prepare_revision3_checkpoint(None, &basis_project)
        .unwrap();
    assert_eq!(basis_first, basis_second);
    publish(&root, &basis_first.head_bytes);

    let artifact_bytes = opaque_canonical_artifact_bytes(REAL_GOLDEN_ARTIFACT_BYTES);
    let imported = store
        .import_quest_collision_artifact_v1(&artifact_bytes, Some(&basis_first.head))
        .unwrap();
    assert_eq!(
        imported.artifact.byte_len,
        REAL_GOLDEN_ARTIFACT_BYTES as u64
    );
    assert_eq!(
        imported.asset_meta.media_type,
        QUEST_COLLISION_ARTIFACT_MEDIA_TYPE
    );
    let project = quest_project(
        basis_first.head.snapshot.clone(),
        imported.artifact.clone(),
        imported.asset_meta.clone(),
    );
    let quest_entity_bytes = serde_json::to_vec(&project.entities[&entity_id(10)]).unwrap();
    assert!(quest_entity_bytes.len() < MAX_REVISION3_ENTITY_JSON_BYTES);
    assert!(artifact_bytes.len() > MAX_REVISION3_ENTITY_JSON_BYTES);
    assert_eq!(
        store
            .read_indexed_quest_collision_artifact_v1(&project.asset_store, &imported.artifact,)
            .unwrap(),
        artifact_bytes
    );

    let first = store
        .prepare_revision3_checkpoint(Some(&basis_first.head), &project)
        .unwrap();
    let second = store
        .prepare_revision3_checkpoint(Some(&basis_first.head), &project)
        .unwrap();
    assert_eq!(first, second);
    let reopened = store
        .open_revision3_head_bytes(&first.head_bytes, AssetVerification::Full)
        .unwrap();
    assert_eq!(reopened.project, project);
    assert_eq!(reopened.head, first.head);

    publish(&root, &first.head_bytes);
    assert_eq!(
        store
            .open_current_revision3(AssetVerification::Full)
            .unwrap()
            .project,
        project
    );
    let old_basis = store
        .open_revision3_snapshot(&basis_first.head.snapshot, AssetVerification::Full)
        .unwrap();
    assert_eq!(old_basis.project, basis_project);
    assert_eq!(old_basis.head, basis_first.head);

    // The generic Store head dispatcher remains revision-1/2-only.
    assert!(matches!(
        store.open_head_bytes_document(
            &first.head_bytes,
            AssetVerification::Full,
            gore_authoring::ValidationProfile::Experimental,
        ),
        Err(WorkingStoreError::Invariant(message))
            if message.contains("expected 1 or 2")
    ));
}

#[test]
fn artifact_import_deduplicates_rejects_collisions_and_obeys_head_cas() {
    let bytes = opaque_canonical_artifact_bytes(4096);

    let dedupe_root = TestRoot::new("artifact-dedupe");
    let dedupe_store = store(&dedupe_root);
    let first = dedupe_store
        .import_quest_collision_artifact_v1(&bytes, None)
        .unwrap();
    let second = dedupe_store
        .import_quest_collision_artifact_v1(&bytes, None)
        .unwrap();
    assert!(!first.deduplicated);
    assert!(second.deduplicated);
    assert_eq!(first.artifact, second.artifact);

    let collision_root = TestRoot::new("artifact-collision");
    let collision_store = store(&collision_root);
    let imported = collision_store
        .import_quest_collision_artifact_v1(&bytes, None)
        .unwrap();
    fs::write(
        asset_path(collision_root.path(), imported.artifact.sha256),
        vec![b'!'; bytes.len()],
    )
    .unwrap();
    assert!(matches!(
        collision_store.import_quest_collision_artifact_v1(&bytes, None),
        Err(WorkingStoreError::Collision { .. })
    ));

    let head_root = TestRoot::new("artifact-head-cas");
    let head_store = store(&head_root);
    let prepared = head_store
        .prepare_revision3_checkpoint(None, &empty_revision3())
        .unwrap();
    publish(&head_root, &prepared.head_bytes);
    let digest = raw_seal(&bytes).sha256;
    assert!(matches!(
        head_store.import_quest_collision_artifact_v1(&bytes, None),
        Err(WorkingStoreError::HeadConflict { .. })
    ));
    assert!(!asset_path(head_root.path(), digest).exists());
    assert!(head_store
        .import_quest_collision_artifact_v1(&bytes, Some(&prepared.head))
        .is_ok());

    let at_limit = opaque_canonical_artifact_bytes(
        usize::try_from(MAX_QUEST_COLLISION_ARTIFACT_BYTES).unwrap(),
    );
    assert!(head_store
        .import_quest_collision_artifact_v1(&at_limit, Some(&prepared.head))
        .is_ok());
    drop(at_limit);
    let oversized = vec![0; usize::try_from(MAX_QUEST_COLLISION_ARTIFACT_BYTES).unwrap() + 1];
    assert!(matches!(
        head_store.import_quest_collision_artifact_v1(&oversized, Some(&prepared.head)),
        Err(WorkingStoreError::LimitExceeded {
            kind: "Quest collision artifact bytes",
            actual,
            limit: MAX_QUEST_COLLISION_ARTIFACT_BYTES,
        }) if actual == MAX_QUEST_COLLISION_ARTIFACT_BYTES + 1
    ));
}

#[test]
fn indexed_artifact_metadata_fails_before_io_and_cas_is_fully_hashed() {
    let bytes = opaque_canonical_artifact_bytes(2048);
    let artifact = raw_seal(&bytes);
    let exact_meta = AssetMeta {
        byte_len: artifact.byte_len,
        media_type: QUEST_COLLISION_ARTIFACT_MEDIA_TYPE.into(),
    };

    // Removing the root proves these argument/index errors precede filesystem inspection.
    let preflight_root = TestRoot::new("artifact-preflight-order");
    let preflight_store = store(&preflight_root);
    fs::remove_dir_all(preflight_root.path()).unwrap();
    assert!(matches!(
        preflight_store.read_indexed_quest_collision_artifact_v1(
            &AssetStoreIndex::default(),
            &artifact,
        ),
        Err(WorkingStoreError::Invariant(message)) if message.contains("absent from the supplied asset index")
    ));
    let wrong_len = AssetStoreIndex {
        assets: BTreeMap::from([(
            artifact.sha256,
            AssetMeta {
                byte_len: artifact.byte_len + 1,
                ..exact_meta.clone()
            },
        )]),
    };
    assert!(matches!(
        preflight_store.read_indexed_quest_collision_artifact_v1(&wrong_len, &artifact),
        Err(WorkingStoreError::Invariant(message)) if message.contains("index declares")
    ));
    for media_type in ["application/json", ""] {
        let wrong_media = AssetStoreIndex {
            assets: BTreeMap::from([(
                artifact.sha256,
                AssetMeta {
                    media_type: media_type.into(),
                    ..exact_meta.clone()
                },
            )]),
        };
        assert!(matches!(
            preflight_store.read_indexed_quest_collision_artifact_v1(&wrong_media, &artifact),
            Err(WorkingStoreError::Invariant(message)) if message.contains("media type")
        ));
    }
    let oversized = ContentSeal {
        byte_len: MAX_QUEST_COLLISION_ARTIFACT_BYTES + 1,
        sha256: artifact.sha256,
    };
    assert!(matches!(
        preflight_store
            .read_indexed_quest_collision_artifact_v1(&AssetStoreIndex::default(), &oversized),
        Err(WorkingStoreError::LimitExceeded {
            kind: "Quest collision artifact bytes",
            ..
        })
    ));

    let missing_root = TestRoot::new("artifact-missing-cas");
    let missing_store = store(&missing_root);
    let exact_index = AssetStoreIndex {
        assets: BTreeMap::from([(artifact.sha256, exact_meta.clone())]),
    };
    assert!(matches!(
        missing_store.read_indexed_quest_collision_artifact_v1(&exact_index, &artifact),
        Err(WorkingStoreError::MissingObject(_))
    ));

    let prepare_root = TestRoot::new("artifact-prepare-preflight");
    let prepare_store = store(&prepare_root);
    let basis = prepare_store
        .prepare_revision3_checkpoint(None, &empty_revision3())
        .unwrap();
    publish(&prepare_root, &basis.head_bytes);
    let missing_artifact_project = quest_project(
        basis.head.snapshot.clone(),
        artifact.clone(),
        exact_meta.clone(),
    );
    assert!(matches!(
        prepare_store.prepare_revision3_checkpoint(Some(&basis.head), &missing_artifact_project),
        Err(WorkingStoreError::MissingObject(_))
    ));
    assert!(!prepare_root.path().join("entities").exists());

    let corrupt_root = TestRoot::new("artifact-corrupt-cas");
    let corrupt_store = store(&corrupt_root);
    let path = asset_path(corrupt_root.path(), artifact.sha256);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, vec![b'?'; bytes.len()]).unwrap();
    assert!(matches!(
        corrupt_store.read_indexed_quest_collision_artifact_v1(&exact_index, &artifact),
        Err(WorkingStoreError::SealMismatch { .. })
    ));

    let valid_root = TestRoot::new("artifact-full-read");
    let valid_store = store(&valid_root);
    let imported = valid_store
        .import_quest_collision_artifact_v1(&bytes, None)
        .unwrap();
    let valid_index = AssetStoreIndex {
        assets: BTreeMap::from([(imported.artifact.sha256, imported.asset_meta)]),
    };
    assert_eq!(
        valid_store
            .read_indexed_quest_collision_artifact_v1(&valid_index, &imported.artifact)
            .unwrap(),
        bytes
    );
}

#[test]
fn basis_snapshot_must_be_canonical_revision3_not_merely_a_valid_sealed_snapshot() {
    let artifact_bytes = opaque_canonical_artifact_bytes(4096);

    let cross_revision_root = TestRoot::new("basis-cross-revision");
    let cross_revision_store = store(&cross_revision_root);
    let revision1 = cross_revision_store
        .prepare_checkpoint(None, &empty_revision1(), ValidationProfile::Production)
        .unwrap();
    let imported = cross_revision_store
        .import_quest_collision_artifact_v1(&artifact_bytes, None)
        .unwrap();
    let project = quest_project(
        revision1.head.snapshot,
        imported.artifact,
        imported.asset_meta,
    );
    assert!(matches!(
        cross_revision_store.prepare_revision3_checkpoint(None, &project),
        Err(WorkingStoreError::InvalidJson {
            kind: "revision-3 basis snapshot",
            ..
        })
    ));
    assert!(!cross_revision_root.path().join("entities").exists());

    let noncanonical_root = TestRoot::new("basis-noncanonical");
    let noncanonical_store = store(&noncanonical_root);
    let canonical_basis = noncanonical_store
        .prepare_revision3_checkpoint(None, &empty_revision3())
        .unwrap();
    let canonical_path = snapshot_path(
        noncanonical_root.path(),
        canonical_basis.head.snapshot.sha256,
    );
    let mut noncanonical_bytes = vec![b' '];
    noncanonical_bytes.extend(fs::read(canonical_path).unwrap());
    let noncanonical_seal = raw_seal(&noncanonical_bytes);
    let noncanonical_path = snapshot_path(noncanonical_root.path(), noncanonical_seal.sha256);
    fs::create_dir_all(noncanonical_path.parent().unwrap()).unwrap();
    fs::write(noncanonical_path, noncanonical_bytes).unwrap();
    let imported = noncanonical_store
        .import_quest_collision_artifact_v1(&artifact_bytes, None)
        .unwrap();
    let project = quest_project(noncanonical_seal, imported.artifact, imported.asset_meta);
    assert!(matches!(
        noncanonical_store.prepare_revision3_checkpoint(None, &project),
        Err(WorkingStoreError::NonCanonicalJson {
            kind: "revision-3 basis snapshot",
        })
    ));
    assert!(!noncanonical_root.path().join("entities").exists());
}
