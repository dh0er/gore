use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use gore_authoring::{
    AssetStoreIndex, AssetVerification, ContentSeal, FormatV2, GameGenerationAnchor, ProjectId,
    ProjectMeta, ProjectRevision3, Revision3HistoryErrorV1, Revision3SnapshotManifest,
    SchemaRevisionV3, Sha256Digest, WorkingHead, WorkingProjectStore, WorkingStoreError,
    WorkingStoreFormat, WorkingStoreLimits, MAX_REVISION3_BASE_SNAPSHOT_BYTES,
    MAX_REVISION3_HISTORY_MANIFEST_BYTES_V1, MAX_REVISION3_HISTORY_PARENT_RECORDS_V1,
    MAX_REVISION3_SNAPSHOT_BYTES, REVISION3_HISTORY_SNAPSHOT_RESERVE_BYTES_V1,
};
use sha2::{Digest as _, Sha256};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);

impl TestRoot {
    fn new(label: &str) -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "gore-authoring-r3-history-{label}-{}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        let _ = fs::remove_file(path.with_extension("goremod"));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn export_path(&self) -> PathBuf {
        self.0.with_extension("goremod")
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
        let _ = fs::remove_file(self.export_path());
    }
}

fn store(root: &TestRoot) -> WorkingProjectStore {
    WorkingProjectStore::at(root.path(), WorkingStoreLimits::default()).unwrap()
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

fn target(value: u8) -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: seal(value, 171_698_176),
    }
}

fn project(project_id: u8, revision: u64, name: &str) -> ProjectRevision3 {
    ProjectRevision3 {
        format: FormatV2,
        schema_revision: SchemaRevisionV3,
        project_id: ProjectId::from_bytes([project_id; 16]),
        revision,
        meta: ProjectMeta {
            name: name.to_owned(),
            version: "0.1.0".to_owned(),
            author: "history tests".to_owned(),
        },
        target: target(1),
        authoring_locales: BTreeSet::new(),
        entities: BTreeMap::new(),
        asset_store: AssetStoreIndex::default(),
    }
}

fn publish(root: &TestRoot, head_bytes: &[u8]) {
    fs::write(root.path().join("gore-project.json"), head_bytes).unwrap();
}

fn snapshot_path(root: &TestRoot, head: &WorkingHead) -> PathBuf {
    let hex = head.snapshot.sha256.to_string();
    root.path()
        .join("snapshots")
        .join("sha256")
        .join(&hex[..2])
        .join(format!("{}.json", &hex[2..]))
}

fn snapshot_manifest(root: &TestRoot, head: &WorkingHead) -> Revision3SnapshotManifest {
    serde_json::from_slice(&fs::read(snapshot_path(root, head)).unwrap()).unwrap()
}

fn install_snapshot_manifest(root: &TestRoot, manifest: &Revision3SnapshotManifest) -> WorkingHead {
    let bytes = serde_json::to_vec(manifest).unwrap();
    let snapshot = raw_seal(&bytes);
    let head = WorkingHead {
        store_format: WorkingStoreFormat,
        snapshot,
    };
    let path = snapshot_path(root, &head);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
    head
}

fn count_files(path: &Path) -> usize {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .map(|entry| {
            let path = entry.unwrap().path();
            if path.is_dir() {
                count_files(&path)
            } else {
                1
            }
        })
        .sum()
}

fn legacy_root_snapshot_bytes() -> Vec<u8> {
    format!(
        concat!(
            "{{\"store_format\":1,\"format\":2,\"schema_revision\":3,",
            "\"project_id\":\"{}\",\"revision\":7,",
            "\"meta\":{{\"name\":\"Legacy root\",\"version\":\"0.1.0\",",
            "\"author\":\"history tests\"}},",
            "\"target\":{{\"executable\":{{\"byte_len\":171698176,\"sha256\":\"{}\"}}}},",
            "\"authoring_locales\":[],\"entities\":{{}},\"asset_store\":{{\"assets\":{{}}}}}}"
        ),
        "03".repeat(16),
        "01".repeat(32),
    )
    .into_bytes()
}

fn snapshot_member_name(head: &WorkingHead) -> String {
    let hex = head.snapshot.sha256.to_string();
    format!("store/snapshots/sha256/{}/{}.json", &hex[..2], &hex[2..])
}

#[test]
fn legacy_root_bytes_stay_closed_and_first_successor_authenticates_parent() {
    let root = TestRoot::new("legacy-root");
    let store = store(&root);
    let root_project = project(3, 7, "Legacy root");
    let first = store
        .prepare_revision3_checkpoint(None, &root_project)
        .unwrap();
    let repeated = store
        .prepare_revision3_checkpoint(None, &root_project)
        .unwrap();
    assert_eq!(first, repeated);

    let root_bytes = fs::read(snapshot_path(&root, &first.head)).unwrap();
    let pinned_legacy_bytes = legacy_root_snapshot_bytes();
    assert_eq!(root_bytes, pinned_legacy_bytes);
    assert_eq!(first.head.snapshot, raw_seal(&pinned_legacy_bytes));
    assert!(!root_bytes
        .windows(b"\"history\":".len())
        .any(|window| window == b"\"history\":"));
    let root_manifest: Revision3SnapshotManifest = serde_json::from_slice(&root_bytes).unwrap();
    assert!(root_manifest.history.is_none());
    assert_eq!(serde_json::to_vec(&root_manifest).unwrap(), root_bytes);

    publish(&root, &first.head_bytes);
    let mut successor_project = root_project.clone();
    successor_project.revision += 1;
    successor_project.meta.name = "Authenticated successor".to_owned();
    let successor = store
        .prepare_revision3_checkpoint(Some(&first.head), &successor_project)
        .unwrap();
    assert_eq!(store.current_head().unwrap(), Some(first.head.clone()));
    let successor_history = snapshot_manifest(&root, &successor.head).history.unwrap();
    assert!(!successor_history.history_truncated);
    assert_eq!(successor_history.prior_checkpoints.len(), 1);
    let parent = &successor_history.prior_checkpoints[0];
    assert_eq!(parent.head, first.head);
    assert_eq!(parent.project_id, root_project.project_id);
    assert_eq!(parent.project_revision, root_project.revision);
    assert_eq!(parent.target, root_project.target);

    publish(&root, &successor.head_bytes);
    let history = store
        .list_current_revision3_history_v1(&successor.head)
        .unwrap();
    assert_eq!(history.basis_head, successor.head);
    assert_eq!(history.current.project_revision, 8);
    assert_eq!(history.current.meta.name, "Authenticated successor");
    assert_eq!(history.parents.len(), 1);
    assert_eq!(history.parents[0].head, first.head);
    assert_eq!(history.parents[0].project_revision, 7);
    assert!(!history.history_truncated);
    let no_op = store
        .prepare_revision3_checkpoint(Some(&successor.head), &successor_project)
        .unwrap();
    assert_eq!(no_op, successor);
}

#[test]
fn history_reserve_allows_a_full_base_snapshot_successor_and_closes_the_final_cap() {
    let root = TestRoot::new("snapshot-reserve");
    let store = store(&root);
    let mut root_project = project(12, 7, "Full base snapshot");
    root_project.meta.author.clear();
    let probe = store
        .prepare_revision3_checkpoint(None, &root_project)
        .unwrap();
    let base_cap = MAX_REVISION3_BASE_SNAPSHOT_BYTES as usize;
    let probe_len = probe.head.snapshot.byte_len as usize;
    assert!(probe_len < base_cap);
    root_project.meta.author = "x".repeat(base_cap - probe_len);

    let full_base = store
        .prepare_revision3_checkpoint(None, &root_project)
        .unwrap();
    assert_eq!(
        full_base.head.snapshot.byte_len,
        MAX_REVISION3_BASE_SNAPSHOT_BYTES
    );
    publish(&root, &full_base.head_bytes);

    root_project.revision += 1;
    let successor = store
        .prepare_revision3_checkpoint(Some(&full_base.head), &root_project)
        .unwrap();
    assert!(successor.head.snapshot.byte_len > MAX_REVISION3_BASE_SNAPSHOT_BYTES);
    assert!(successor.head.snapshot.byte_len <= MAX_REVISION3_SNAPSHOT_BYTES);
    assert!(
        successor.head.snapshot.byte_len - MAX_REVISION3_BASE_SNAPSHOT_BYTES
            < REVISION3_HISTORY_SNAPSHOT_RESERVE_BYTES_V1 as u64
    );
    publish(&root, &successor.head_bytes);
    let history = store
        .list_current_revision3_history_v1(&successor.head)
        .unwrap();
    assert_eq!(history.parents[0].head, full_base.head);
    root_project.revision += 1;
    let second_successor = store
        .prepare_revision3_checkpoint(Some(&successor.head), &root_project)
        .unwrap();
    assert!(second_successor.head.snapshot.byte_len <= MAX_REVISION3_SNAPSHOT_BYTES);

    let oversized_head = serde_json::to_vec(&WorkingHead {
        store_format: WorkingStoreFormat,
        snapshot: ContentSeal {
            byte_len: MAX_REVISION3_SNAPSHOT_BYTES + 1,
            sha256: Sha256Digest::from_bytes([0x77; 32]),
        },
    })
    .unwrap();
    assert!(matches!(
        store.open_revision3_head_bytes(&oversized_head, AssetVerification::Full),
        Err(WorkingStoreError::LimitExceeded {
            kind: "revision-3 snapshot",
            limit: MAX_REVISION3_SNAPSHOT_BYTES,
            ..
        })
    ));
}

#[test]
fn stricter_custom_base_limit_still_reopens_and_advances_reserved_history_snapshots() {
    let root = TestRoot::new("custom-snapshot-reserve");
    let default_store = store(&root);
    let mut current_project = project(13, 1, "Custom reserve");
    let first = default_store
        .prepare_revision3_checkpoint(None, &current_project)
        .unwrap();
    publish(&root, &first.head_bytes);

    let base_limit = first.head.snapshot.byte_len as usize;
    let strict_store = WorkingProjectStore::open_existing(
        root.path(),
        WorkingStoreLimits {
            max_snapshot_bytes: base_limit,
            ..WorkingStoreLimits::default()
        },
    )
    .unwrap();
    current_project.revision += 1;
    let successor = strict_store
        .prepare_revision3_checkpoint(Some(&first.head), &current_project)
        .unwrap();
    assert!(successor.head.snapshot.byte_len > base_limit as u64);
    assert!(
        successor.head.snapshot.byte_len
            <= (base_limit + REVISION3_HISTORY_SNAPSHOT_RESERVE_BYTES_V1) as u64
    );
    publish(&root, &successor.head_bytes);
    assert_eq!(
        strict_store
            .list_current_revision3_history_v1(&successor.head)
            .unwrap()
            .parents[0]
            .head,
        first.head
    );

    current_project.revision += 1;
    strict_store
        .prepare_revision3_checkpoint(Some(&successor.head), &current_project)
        .unwrap();
}

#[test]
fn successor_rejects_revision_identity_and_target_drift_before_immutable_writes() {
    let root = TestRoot::new("successor-conflicts");
    let store = store(&root);
    let basis_project = project(4, 10, "Basis");
    let basis = store
        .prepare_revision3_checkpoint(None, &basis_project)
        .unwrap();
    publish(&root, &basis.head_bytes);
    let initial_files = count_files(&root.path().join("snapshots"));
    let initial_store_files = count_files(root.path());
    let exact_no_op = store
        .prepare_revision3_checkpoint(Some(&basis.head), &basis_project)
        .unwrap();
    assert_eq!(exact_no_op, basis);
    assert_eq!(count_files(&root.path().join("snapshots")), initial_files);
    assert_eq!(count_files(root.path()), initial_store_files);

    let mut candidates = Vec::new();
    let mut changed_same_revision = basis_project.clone();
    changed_same_revision.meta.name = "Changed without revision".to_owned();
    candidates.push(changed_same_revision);
    let mut gap = basis_project.clone();
    gap.revision += 2;
    candidates.push(gap);
    let mut wrong_id = basis_project.clone();
    wrong_id.revision += 1;
    wrong_id.project_id = ProjectId::from_bytes([9; 16]);
    candidates.push(wrong_id);
    let mut wrong_target = basis_project.clone();
    wrong_target.revision += 1;
    wrong_target.target = target(8);
    candidates.push(wrong_target);

    for candidate in candidates {
        assert!(matches!(
            store.prepare_revision3_checkpoint(Some(&basis.head), &candidate),
            Err(WorkingStoreError::Invariant(_))
        ));
        assert_eq!(count_files(&root.path().join("snapshots")), initial_files);
        assert_eq!(store.current_head().unwrap(), Some(basis.head.clone()));
    }
}

#[test]
fn history_lists_every_retained_member_newest_first_and_old_damage_does_not_break_current_open() {
    let root = TestRoot::new("bounded-and-damaged");
    let store = store(&root);
    let mut current_project = project(5, 20, "Revision 20");
    let first = store
        .prepare_revision3_checkpoint(None, &current_project)
        .unwrap();
    publish(&root, &first.head_bytes);
    let mut heads = vec![first];
    for revision in 21..=23 {
        current_project.revision = revision;
        current_project.meta.name = format!("Revision {revision}");
        let prepared = store
            .prepare_revision3_checkpoint(Some(&heads.last().unwrap().head), &current_project)
            .unwrap();
        publish(&root, &prepared.head_bytes);
        heads.push(prepared);
    }
    let current = heads.last().unwrap();

    let page = store
        .list_current_revision3_history_v1(&current.head)
        .unwrap();
    assert_eq!(
        page.parents
            .iter()
            .map(|entry| entry.project_revision)
            .collect::<Vec<_>>(),
        vec![22, 21, 20]
    );
    assert!(!page.history_truncated);
    assert!(matches!(
        store.list_current_revision3_history_v1(&heads[1].head),
        Err(Revision3HistoryErrorV1::Store(
            WorkingStoreError::HeadConflict { .. }
        ))
    ));

    fs::remove_file(snapshot_path(&root, &heads[0].head)).unwrap();
    let opened = store
        .open_current_revision3(AssetVerification::Full)
        .unwrap();
    assert_eq!(opened.head, current.head);
    assert_eq!(opened.project, current_project);
    assert!(matches!(
        store.list_current_revision3_history_v1(&current.head),
        Err(Revision3HistoryErrorV1::Store(
            WorkingStoreError::MissingObject(_)
        ))
    ));
}

#[test]
fn history_rejects_aggregate_manifest_work_above_the_closed_byte_budget() {
    let root = TestRoot::new("aggregate-manifest-budget");
    let store = store(&root);
    let mut current_project = project(11, 1, "Revision 1");
    let first = store
        .prepare_revision3_checkpoint(None, &current_project)
        .unwrap();
    publish(&root, &first.head_bytes);
    let mut current = first;
    for revision in 2..=6 {
        current_project.revision = revision;
        current_project.meta.name = format!("Revision {revision}");
        current = store
            .prepare_revision3_checkpoint(Some(&current.head), &current_project)
            .unwrap();
        publish(&root, &current.head_bytes);
    }

    let mut forged_manifest = snapshot_manifest(&root, &current.head);
    for retained in &mut forged_manifest.history.as_mut().unwrap().prior_checkpoints {
        retained.head.snapshot.byte_len = WorkingStoreLimits::default().max_snapshot_bytes as u64;
    }
    let forged_head = install_snapshot_manifest(&root, &forged_manifest);
    publish(&root, &serde_json::to_vec(&forged_head).unwrap());

    assert!(matches!(
        store.open_current_revision3(AssetVerification::Full),
        Err(WorkingStoreError::LimitExceeded {
            kind: "aggregate revision-3 retained history manifest bytes",
            limit: MAX_REVISION3_HISTORY_MANIFEST_BYTES_V1,
            ..
        })
    ));
    assert!(matches!(
        store.list_current_revision3_history_v1(&forged_head),
        Err(Revision3HistoryErrorV1::Store(
            WorkingStoreError::LimitExceeded {
                kind: "aggregate revision-3 retained history manifest bytes",
                limit: MAX_REVISION3_HISTORY_MANIFEST_BYTES_V1,
                ..
            }
        ))
    ));
}

#[test]
fn shallow_current_open_does_not_grant_forged_retained_identity_history_authority() {
    let root = TestRoot::new("forged-parent");
    let store = store(&root);
    let foreign_project = project(90, 7, "Foreign orphan");
    let foreign = store
        .prepare_revision3_checkpoint(None, &foreign_project)
        .unwrap();
    let basis_project = project(9, 7, "Real basis");
    let basis = store
        .prepare_revision3_checkpoint(None, &basis_project)
        .unwrap();
    publish(&root, &basis.head_bytes);
    let mut current_project = basis_project;
    current_project.revision = 8;
    current_project.meta.name = "Real current".to_owned();
    let current = store
        .prepare_revision3_checkpoint(Some(&basis.head), &current_project)
        .unwrap();

    let mut forged_manifest = snapshot_manifest(&root, &current.head);
    forged_manifest.history.as_mut().unwrap().prior_checkpoints[0].head = foreign.head;
    let forged_head = install_snapshot_manifest(&root, &forged_manifest);
    publish(&root, &serde_json::to_vec(&forged_head).unwrap());

    // Ordinary current authoring content remains independently usable; it makes no history claim.
    let opened = store
        .open_current_revision3(AssetVerification::Full)
        .unwrap();
    assert_eq!(opened.head, forged_head);
    assert_eq!(opened.project, current_project);
    // History follows the sealed edge and independently rejects the actual foreign snapshot.
    assert!(matches!(
        store.list_current_revision3_history_v1(&forged_head),
        Err(Revision3HistoryErrorV1::InvalidLineage(message))
            if message.contains("disagrees with its sealed snapshot")
    ));
}

#[test]
fn bounded_vector_truncates_at_255_priors_and_export_never_revives_dropped_history() {
    let root = TestRoot::new("bounded-vector");
    let store = store(&root);
    let mut current_project = project(10, 1, "Revision 1");
    let first = store
        .prepare_revision3_checkpoint(None, &current_project)
        .unwrap();
    publish(&root, &first.head_bytes);
    let mut heads = vec![first];

    for revision in 2..=257 {
        current_project.revision = revision;
        current_project.meta.name = format!("Revision {revision}");
        let prepared = store
            .prepare_revision3_checkpoint(Some(&heads.last().unwrap().head), &current_project)
            .unwrap();
        if revision == 256 {
            let at_capacity = snapshot_manifest(&root, &prepared.head).history.unwrap();
            assert_eq!(
                at_capacity.prior_checkpoints.len(),
                MAX_REVISION3_HISTORY_PARENT_RECORDS_V1
            );
            assert!(!at_capacity.history_truncated);
        }
        publish(&root, &prepared.head_bytes);
        heads.push(prepared);
    }

    let current = heads.last().unwrap();
    let retained = snapshot_manifest(&root, &current.head).history.unwrap();
    assert_eq!(
        retained.prior_checkpoints.len(),
        MAX_REVISION3_HISTORY_PARENT_RECORDS_V1
    );
    assert!(retained.history_truncated);
    assert_eq!(retained.prior_checkpoints[0].head, heads[255].head);
    assert_eq!(
        retained.prior_checkpoints.last().unwrap().head,
        heads[1].head
    );

    let history = store
        .list_current_revision3_history_v1(&current.head)
        .unwrap();
    assert_eq!(
        history.parents.len(),
        MAX_REVISION3_HISTORY_PARENT_RECORDS_V1
    );
    assert!(history.history_truncated);
    assert_eq!(history.current.project_revision, 257);
    assert_eq!(history.parents[0].project_revision, 256);
    assert_eq!(history.parents.last().unwrap().project_revision, 2);
    assert!(matches!(
        store.prepare_revision3_history_restore_v1(&current.head, &heads[0].head),
        Err(Revision3HistoryErrorV1::TargetNotReachable { .. })
    ));

    let export = store
        .export_current_revision3_exact_snapshot_v2(&current.head, root.export_path())
        .unwrap();
    assert_eq!(
        export.receipt().closure.snapshot_objects,
        (MAX_REVISION3_HISTORY_PARENT_RECORDS_V1 + 1) as u64
    );
    let mut archive = zip::ZipArchive::new(fs::File::open(root.export_path()).unwrap()).unwrap();
    let snapshot_members = (0..archive.len())
        .map(|index| archive.by_index(index).unwrap().name().to_owned())
        .filter(|name| name.starts_with("store/snapshots/"))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        snapshot_members.len(),
        MAX_REVISION3_HISTORY_PARENT_RECORDS_V1 + 1
    );
    assert!(!snapshot_members.contains(&snapshot_member_name(&heads[0].head)));
    for retained in &heads[1..] {
        assert!(snapshot_members.contains(&snapshot_member_name(&retained.head)));
    }
}

#[test]
fn restore_accepts_only_reachable_ancestors_and_prepares_new_current_plus_one() {
    let root = TestRoot::new("restore");
    let store = store(&root);
    let mut current_project = project(6, 7, "Original content");
    let original = store
        .prepare_revision3_checkpoint(None, &current_project)
        .unwrap();
    publish(&root, &original.head_bytes);

    current_project.revision = 8;
    current_project.meta.name = "Middle content".to_owned();
    let middle = store
        .prepare_revision3_checkpoint(Some(&original.head), &current_project)
        .unwrap();
    publish(&root, &middle.head_bytes);

    current_project.revision = 9;
    current_project.meta.name = "Current content".to_owned();
    let current = store
        .prepare_revision3_checkpoint(Some(&middle.head), &current_project)
        .unwrap();
    publish(&root, &current.head_bytes);

    let mut orphan_project = current_project.clone();
    orphan_project.revision = 10;
    orphan_project.meta.name = "Unpublished child".to_owned();
    let orphan = store
        .prepare_revision3_checkpoint(Some(&current.head), &orphan_project)
        .unwrap();
    assert!(matches!(
        store.prepare_revision3_history_restore_v1(&current.head, &orphan.head),
        Err(Revision3HistoryErrorV1::TargetNotReachable { .. })
    ));
    assert!(matches!(
        store.prepare_revision3_history_restore_v1(&current.head, &current.head),
        Err(Revision3HistoryErrorV1::TargetNotReachable { .. })
    ));

    let restored = store
        .prepare_revision3_history_restore_v1(&current.head, &original.head)
        .unwrap();
    assert_eq!(restored.basis_head, current.head);
    assert_eq!(restored.restored_from.head, original.head);
    assert_eq!(restored.restored_from.project_revision, 7);
    assert_eq!(restored.project.revision, 10);
    assert_eq!(restored.project.meta.name, "Original content");
    assert_eq!(store.current_head().unwrap(), Some(current.head.clone()));

    let reopened = store
        .open_revision3_head_bytes(&restored.checkpoint.head_bytes, AssetVerification::Full)
        .unwrap();
    assert_eq!(reopened.head, restored.checkpoint.head);
    assert_eq!(reopened.project, restored.project);
    let restored_history = snapshot_manifest(&root, &restored.checkpoint.head)
        .history
        .unwrap();
    let parent = &restored_history.prior_checkpoints[0];
    assert_eq!(parent.head, current.head);
    assert_eq!(parent.project_revision, 9);

    publish(&root, &restored.checkpoint.head_bytes);
    let history = store
        .list_current_revision3_history_v1(&restored.checkpoint.head)
        .unwrap();
    assert_eq!(history.current.project_revision, 10);
    assert_eq!(history.current.meta.name, "Original content");
    assert_eq!(history.parents[0].head, current.head);
    assert_eq!(history.parents[0].meta.name, "Current content");
}

#[test]
fn restore_revision_overflow_fails_without_a_candidate() {
    let root = TestRoot::new("restore-overflow");
    let store = store(&root);
    let first_project = project(7, u64::MAX - 1, "Before overflow");
    let first = store
        .prepare_revision3_checkpoint(None, &first_project)
        .unwrap();
    publish(&root, &first.head_bytes);
    let mut current_project = first_project;
    current_project.revision = u64::MAX;
    current_project.meta.name = "At overflow".to_owned();
    let current = store
        .prepare_revision3_checkpoint(Some(&first.head), &current_project)
        .unwrap();
    publish(&root, &current.head_bytes);
    let snapshots_before = count_files(&root.path().join("snapshots"));

    assert!(matches!(
        store.prepare_revision3_history_restore_v1(&current.head, &first.head),
        Err(Revision3HistoryErrorV1::ProjectRevisionOverflow { current: u64::MAX })
    ));
    assert_eq!(
        count_files(&root.path().join("snapshots")),
        snapshots_before
    );
    assert_eq!(store.current_head().unwrap(), Some(current.head));
}

#[test]
fn exact_export_includes_reachable_history_but_excludes_unpublished_children() {
    let root = TestRoot::new("export-closure");
    let store = store(&root);
    let mut current_project = project(8, 1, "Export root");
    let first = store
        .prepare_revision3_checkpoint(None, &current_project)
        .unwrap();
    publish(&root, &first.head_bytes);
    current_project.revision = 2;
    current_project.meta.name = "Export current".to_owned();
    let current = store
        .prepare_revision3_checkpoint(Some(&first.head), &current_project)
        .unwrap();
    publish(&root, &current.head_bytes);

    let mut orphan_project = current_project;
    orphan_project.revision = 3;
    orphan_project.meta.name = "Unpublished export child".to_owned();
    let orphan = store
        .prepare_revision3_checkpoint(Some(&current.head), &orphan_project)
        .unwrap();
    let export = store
        .export_current_revision3_exact_snapshot_v2(&current.head, root.export_path())
        .unwrap();
    assert_eq!(export.receipt().closure.snapshot_objects, 2);

    let archive_bytes = fs::read(root.export_path()).unwrap();
    let orphan_digest = orphan.head.snapshot.sha256.to_string();
    assert!(!archive_bytes
        .windows(orphan_digest.len())
        .any(|window| window == orphan_digest.as_bytes()));
}
